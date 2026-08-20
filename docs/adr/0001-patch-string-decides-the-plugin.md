# ADR 0001: patch 文字列がプラグインを決める

- 状態: 採用（2026-08-20）
- 関連: [0003](0003-mml-patch-key.md) / [0006](0006-per-profile-relative-base.md) /
  play-server `docs/adr/0007-patch-string-decides-the-plugin.md`（判別規則の実装本体）

## 決定

音色は **cartridge を仮想ディレクトリに見立てた相対パス文字列**で表す。
どのプラグインで鳴らすかは、**この文字列の形だけ**から決める。

```
Surge XT : "patches_factory/Pads/Pad 1.fxp"
Dexed    : "SynprezFM/SynprezFM_01.syx/01 Say Again."
           └ サブディレクトリ ┘└ cartridge ┘└ program ┘
                                            └ 0-based index を 2 桁 ┘
```

- config に `[tracks.*]` / `[instances.*]` のような「track → プラグイン」マッピングは**作らない**
- 判別は「path のコンポーネントに拡張子 `.syx` のものがあるか」1 本
- **「プラグイン名を patch 文字列へ明示的に入れる」仕様変更には、いま踏み込まない**

## 理由

TUI 側は patch 文字列を**不透明な表示パスとしてしか扱っていない**。実際の依存は 3 つだけで
（末尾コンポーネントの stem 一致、先頭ディレクトリ = category、相対表示名づくり）、
いずれもこの文字列形式でそのまま動く。

結果として notepad / mml-overlay / daw / grid-sequencer / history / disk_cache /
realtime-ipc が**すべて無改修**になった。当初「別次元の変更」と見積もられていた 4 領域
（cache key / IPC / DAW model / server instance pool）は **4 → 1** に減っている。

理由は単純で、patch 文字列がプラグインを決めるなら、**patch 文字列を運んでいる既存の経路**
（MML 先頭 JSON、history、SHM の `patch` フィールド、DAW セル）が
**そのままプラグイン情報も運んでいる**ことになるから。

## 承知したうえで受け入れた弱点

- 型で守られない。`.syx` を名前に含む Surge patch があれば誤判定する（実質ありえない）
- cartridge の content hash を識別子に混ぜられない
- **文字列だけでは「どのプラグインの音色か」を判別できない**場合がある。
  同じ形を扱うプラグインが 2 つ載ると区別できない

## 踏み込まない理由（未解決の論点として残す）

明示的にプラグイン名を patch 文字列へ入れると、**display 文字列＝永続 ID が変わる**。
保存済みの MML / history / DAW セル / grid session が全部指し先を失うので、移行が発生する。
現状の形のままで判別できている以上、移行のコストに見合わない。

## 壊れたら気づく場所

- `patches/src/layout/tests.rs::a_prefixless_surge_name_reads_the_same_either_way`
  — prefix 抜きで保存された Surge の名前が `PatchLayout::Cartridge` に落ちても
  結果が変わらないこと。この同値が崩れると保存済みの patch 名がカテゴリを失う
- `tui-core/src/patches/tests.rs` — カタログにプラグインが増えても display がビット単位で同じこと
