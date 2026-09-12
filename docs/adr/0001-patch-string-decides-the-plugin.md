# ADR 0001: patch 文字列がプラグインを決める

- 状態: 採用
- 関連: [0003](0003-mml-patch-key.md) / [0006](0006-per-profile-relative-base.md) /
  play-server `docs/adr/0007-patch-string-decides-the-plugin.md`（判別規則の実装本体）

## 決定

音色は **cartridge を仮想ディレクトリに見立てた相対パス文字列**で表す。
どのプラグインで鳴らすかは、**この文字列の形だけ**から決める。

```
Surge XT   : "patches_factory/Pads/Pad 1.fxp"
Dexed      : "SynprezFM/SynprezFM_01.syx/01 Say Again."
             └ サブディレクトリ ┘└ cartridge ┘└ program ┘
                                              └ 0-based index を 2 桁 ┘
Vaporizer2 : "AR Accent Arp.vvp"
             └ 先頭 2 文字がカテゴリコード（AR = Arpeggio）┘
Floe       : "Celtic Harp Factory Presets/Realistic Celtic Harp.floe-preset"
Sforzando  : "sfz/Virtual-Playing-Orchestra3/Woodwinds/flute-SOLO-sustain.sfz"
```

- config に `[tracks.*]` / `[instances.*]` のような「track → プラグイン」マッピングは**作らない**
- 判別は**拡張子だけ**。`.syx` → Dexed / `.vvp` → Vaporizer2 /
  `.floe-preset` → Floe / `.sfz` → Sforzando / それ以外 → Surge XT
  （実体は play-server の `patch_form_of_path()` 1 本）
- **「プラグイン名を patch 文字列へ明示的に入れる」仕様変更には、いま踏み込まない**

## 理由

TUI 側は patch 文字列を**不透明な表示パスとしてしか扱っていない**。実際の依存は 3 つだけで
（末尾コンポーネントの stem 一致、先頭ディレクトリ = category、相対表示名づくり）、
いずれもこの文字列形式でそのまま動く。

patch 文字列がプラグインを決めるなら、**patch 文字列を運んでいる既存の経路**
（MML 先頭 JSON、history、SHM の `patch` フィールド、DAW セル）が
**そのままプラグイン情報も運ぶ**。結果として notepad / mml-overlay / daw / grid-sequencer /
history / disk_cache / realtime-ipc は無改修で済む。

## 承知したうえで受け入れた弱点

- 型で守られない。`.syx` を名前に含む Surge patch があれば誤判定する（実質ありえない）
- cartridge の content hash を識別子に混ぜられない
- **同じ形を扱うプラグインが 2 つ載ると区別できない。** `.vvp` は `.fxp` と同じ
  「1 ファイル = 1 音色 = 1 CLAP state」だが、固有の拡張子があったので `PatchForm` に 1 値足すだけで
  display 文字列を変えずに済んだ。`.fxp` を音色にするプラグインがもう 1 つ載る日には同じ手が使えず、
  patch 文字列の形を変えるか、config で「この dir はこのプラグイン」と明示するかの二択になる
  （後者なら永続 ID は保てる）

## 踏み込まない理由（未解決の論点として残す）

明示的にプラグイン名を patch 文字列へ入れると、**display 文字列＝永続 ID が変わる**。
保存済みの MML / history / DAW セル / grid session が全部指し先を失うので、移行が発生する。
現状の形のままで判別できている以上、移行のコストに見合わない。

## 壊れたら気づく場所

- `patches/src/layout/tests.rs::abstract_metadata_keeps_prefixless_categories`
  — prefix 抜きで保存された Surge の名前でも先頭ディレクトリがカテゴリとして
  読めること。この同値が崩れると保存済みの patch 名がカテゴリを失う
- `tui-core/src/patches/tests.rs` — カタログにプラグインが増えても display がビット単位で同じこと
- `tui-core/src/patch_plugins/tests.rs::a_vvp_patch_goes_to_vaporizer2_not_to_the_other_state_file_plugin`
  — **`.vvp` が Surge の添字へ落ちないこと。** ここが落ちると Vaporizer2 の音色が
  Surge のインスタンスへ送られる（play-server 側の照合で落ちるので静かには壊れないが、
  画面からは「選んだのに鳴らない」に見える）
- `tui-core/src/patch_plugins/tests.rs::an_unsupported_patch_does_not_fall_back_to_the_first_plugin`
  — Vaporizer2 を積んでいない環境で `.vvp` が既定プラグインへ黙って落ちないこと
  （`RouteError::Unsupported` で止まる）
- `tui-core/src/patch_plugins/tests.rs::five_plugin_catalog_routes_sfz_only_to_sforzando`
  — `.sfz` が state file の既定プラグインへ落ちないこと
