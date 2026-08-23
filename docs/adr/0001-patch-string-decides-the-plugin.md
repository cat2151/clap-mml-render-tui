# ADR 0001: patch 文字列がプラグインを決める

- 状態: 採用（2026-08-20 / 2026-08-22 に `.vvp`、2026-08-23 に `.floe-preset` と `.sfz` を追加）
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

## 3 つめのプラグイン（Vaporizer2）で 1 回試された（2026-08-22）

上の弱点「**同じ形を扱うプラグインが 2 つ載ると区別できない**」は、Vaporizer2 で
**初めて実害の一歩手前まで来た**。`.vvp` も `.fxp` と同じ「1 ファイル = 1 音色 = 1 CLAP state」
なので、当時 2 値だった `PatchForm` では Surge XT と区別できず、
`.vvp` が Surge のインスタンスへ流れる（＝ Surge がプロセスごと落ちる）ところだった。

**結果としては永続 ID を変えずに解けた。`.vvp` という固有拡張子があったから。**

- `PatchForm` を `{ StateFile, Cartridge, Vvp }` の 3 値へ広げただけ
- **display 文字列は 1 バイトも変わっていない**（保存済み MML / history / DAW セル /
  grid session の移行はゼロ）
- 判別規則は「拡張子で決める」のまま。原則を曲げていない

**それでも論点は残る。** 解けたのは「たまたま拡張子が違った」からで、
`.fxp` を音色にするプラグインがもう 1 つ載る日には同じ手が使えない。
そのときは patch 文字列の形を変えるか、config で「この dir はこのプラグイン」と
明示するかの二択になる（後者なら永続 ID は保てる）。
**論点を消さずに残しておくのはそのため。**

## 壊れたら気づく場所

- `patches/src/layout/tests.rs::a_prefixless_surge_name_reads_the_same_either_way`
  — prefix 抜きで保存された Surge の名前が `PatchLayout::Cartridge` に落ちても
  結果が変わらないこと。この同値が崩れると保存済みの patch 名がカテゴリを失う
- `tui-core/src/patches/tests.rs` — カタログにプラグインが増えても display がビット単位で同じこと
- `tui-core/src/patch_plugins/tests.rs::a_vvp_patch_goes_to_vaporizer2_not_to_the_other_state_file_plugin`
  — **`.vvp` が Surge の添字へ落ちないこと。** ここが落ちると Vaporizer2 の音色が
  Surge のインスタンスへ送られる（play-server 側の照合で落ちるので静かには壊れないが、
  画面からは「選んだのに鳴らない」に見える）
- `tui-core/src/patch_plugins/tests.rs::a_vvp_patch_falls_back_to_the_default_plugin_when_vaporizer2_is_absent`
  — Vaporizer2 を積んでいない環境の倒れ方が変わっていないこと
- `tui-core/src/patch_plugins/tests.rs::five_plugin_catalog_routes_sfz_only_to_sforzando`
  — `.sfz` が state file の既定プラグインへ落ちないこと
