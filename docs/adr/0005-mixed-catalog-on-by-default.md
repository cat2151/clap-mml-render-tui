# ADR 0005: 混在カタログは既定 ON。実在する dir だけ載せる

- 状態: 採用（2026-08-20 / 2026-08-23 に 5 プラグインで再確認）
- 関連: [0004](0004-default-plugin-owns-unspecified-patches.md) / [0006](0006-per-profile-relative-base.md) /
  [0009](0009-offline-entry-map.md)

## 決定

- **config への opt-in は要らない。** インストール済みのプロファイルは既定でカタログに載る
- 載せる前に**プラグイン本体と音色置き場の両方**を実在確認する
- カタログの**先頭は必ず既定プラグイン**
- **wheel も Cycle Random（1 周ごとの自動抽選）もプラグインをまたぐ。制限なし**

`catalog_plugins()`（`cmrt-runtime/src/core_config.rs`）が唯一の分岐点。
`PatchPlugins` / `InProcessPlugins` / `collect_patch_pairs` / `GridRoleFilters` は
すべてこの並びに従う。

## 実在チェックの掛け方

| 対象 | 条件 |
|---|---|
| プラグイン本体 | `plugin_path` が実在するファイルであること |
| 音色置き場 | `patches_dirs` のうち**実在する dir だけ**を残し、1 つも残らなければカタログへ載せない |
| **既定プラグインの音色置き場** | **実在チェックをしない**（下記） |

未インストールのプラグインの既定 dir で `read_dir` が `Err` になり、**一覧全体が失敗する事故**を
先回りで潰すためのチェック。逆に既定プラグインについては、設定に書いた dir が無いのは
**設定ミス**なので、一覧の収集がエラーになる今までどおりの振る舞いを残す。

## 混在しない条件（意図的な 1 つの穴）

`cfg.plugin_path` が空の config では混在させない（既定プラグインだけを返す）。
空は「どのプラグインも指していない」ということで entry をロードできず、既定プラグインを
同定できない以上、同じものを二重に載せていないかも確かめられないため。
実運用の config では必ず埋まるので、効くのは `Config::default()` を土台にするテストだけ。

## 全画面が同じリストを共有する

`collect_patch_pairs()` は起動時に 1 回だけ走り、**全画面が同じリストを共有する**
（`app/src/tui/session.rs`）。つまり notepad / mml-overlay / keyboard の一覧にも Dexed が混ざる。
**だからオフライン経路の entry 引き分け（[0009](0009-offline-entry-map.md)）が前提になる。**

Sforzando もこの経路へ載せる。MML overlay 専用の SFZ 一覧は作らず、notepad / keyboard /
grid sequencer / DAW が同じ `Arc<Mutex<PatchLoadState>>` を読む。番人は
`app/src/tui/tests/sforzando_screens.rs`。

## 罠: テストが開発機に左右される

カタログが「このマシンに何がインストールされているか」に依存するので、
**`Config` を作って `catalog_plugins` / `PatchPlugins::from_config` / `collect_patch_pairs` を
呼ぶテストを新しく書かないこと。カタログを手で並べて渡すこと。**

実際にこれで 9 本落ちた（`daw` の random patch 系がフィクスチャの代わりに実物の音色を掴んだ）。
外から渡せる口を用意してある:

- `catalog_plugins_with(cfg, installed)`（`cmrt-runtime`）
- `SourceSet::from_catalog(cfg, &PatchPlugins)`（`app/src/voicing_sources.rs`）
- `VoicingPolicies { plugins }` を直接組むテストヘルパ（`app/src/tui/voicing/tests.rs`）

## 3 つめ（Vaporizer2）は「config に 1 行書くまで載らない」（2026-08-22）

**プラグイン本体はインストール済み・組み込みプロファイルもある。それでも既定では載らない。**
Vaporizer2 は**音色置き場の既定値を持たない**（置き場がユーザー固有の設定で決まるため。
play-server 側 ADR 0014）ので、上の表の「音色置き場」チェックで `catalog_plugins_with()` が
`continue` する。

`[plugins.Vaporizer2]` に `patches_dirs` を 1 行書くと 3 つめとして載る。
**既定 config の数字が 1 件も動かないのが正しい**（既存ユーザーの一覧を勝手に変えない）。

## 実測（2026-08-20 / release / `active_plugin = 'Dexed'`）

`cmrt patch-roles`: カタログは Dexed（既定）→ Surge XT の 2 つ、Dexed は 1 度しか出ない。
patch 件数 **4064**（= Surge 3008 + Dexed 1056）。
候補数 Chord 1807 / Bass 1464 / Arpeggio 2059 / Free 3313 /
Kick 1106 / Snare 1101 / HiHat 1078 / Percussion 1178。

**Vaporizer2 を足した config**（`--config` で試す）では 3 プラグイン / patch 件数 **4524**
（= 4064 + `.vvp` 460）。候補数は Chord 1983 / Bass 1567 / Arpeggio 2123 / Free 3597。
→ ベースラインと読み方は [0011](0011-verification-and-baselines.md)

## 実測（2026-08-23 / debug / `active_plugin = 'Dexed'`）

実 config に Floe / Sforzando / Vaporizer2 の profile を設定した `cmrt patch-roles` では、
Dexed 1056 / Floe 13 / **Sforzando 583** / Surge XT 3008 / Vaporizer2 460、合計 **5120**。
Sforzando は user bank 529 件と installed bank manifest 登録 54 件だけを載せる。実在する 595 SFZ のうち、
helper / manifest 未登録 12 件はロード不能なので除外し、source notice と log に件数を残す。

## 外したことを 3 経路で見せる（2026-08-22 追記）

**黙って外す倒れ方そのものは変えていない。** 変えたのは「外したことが誰にも見えない」ほう。

きっかけは実際に起きた迷子で、**Vaporizer2 をインストールし、実装も 10 Stage 全部入っているのに、
音色選択に 460 件が 1 件も出ない**というもの。原因は実 config へ `[plugins.Vaporizer2]` を
書いていなかっただけだが、**ログにも画面にも `cmrt patch-roles` にも痕跡が無く**、
`catalog_plugins_with()` の `continue` を知っている人にしか切り分けられなかった。

**「一覧に出てこない」は一覧を見ていて気づけない。**（出ていないものは見えない）
だから通知は一覧の外に置くしかない。

`catalog_plugins_detailed()` が、載せたぶんと**外したぶん＋理由**を 1 回の走査で同時に返す
（`SkippedCatalogPlugin` / `CatalogSkipReason`）。判定と理由を同じ 1 か所で作るのが要点で、
別の関数で書き直すと条件がずれて「一覧には出ないのに『外していません』と言う」状態になる。

| 経路 | 出るもの |
|---|---|
| `cmrt patch-roles` | `[カタログから外したプラグイン]` 欄。**0 件でも「なし」と出す**（「無い」と「数えていない」を区別するため） |
| `log/log.txt` | `patch-load: event=skipped plugin=... reason=no-patches-dirs note="..."`。`event=ready` / `event=error` も同時に新設 |
| 音色選択（4 画面） | MML overlay / grid sequencer / keyboard / notepad。枠の下辺か help 行の上へ 1 行 |

理由は 2 つに分ける。**「書いていない」と「書いたが実在しない」は次の一手が違う**
（後者に「未設定です」と案内すると、書いた本人には直しようがない）:

- `NoPatchDirs` — `patches_dirs` が無い。Vaporizer2 の組み込みがこれ
- `PatchDirsMissing(dirs)` — 書いてあるが 1 つも実在しない。**綴りを間違えた dir を名指しで返す**
- `PatchSourceUnavailable` — plugin adapter がロード可能な source を解決できない。
  config の欠落場所と resolver の診断を同じ行へ出す

**インストールしていないプラグインはここに出てこない。** `installed_plugin_profiles()` の
時点で落ちており、「入れていないものが出ない」は説明の要らない当たり前だから。
出るのは**入っているのに設定不足で外れたもの**だけ。

### 文言と数え方の単一ソース

- 文言: `SkippedCatalogPlugin::notice_line()`。3 経路で書き分けると直すとき片方だけ古くなる
- 数え方: `catalog_notice_lines(cfg)`。**画面側では数えない**

画面が自分で数えると 2 つ壊れる。(1) 案内が出ない画面が 1 つ残っても気づけない
（**案内が無いこと自体が症状**なので見ただけでは分からない）。(2) 上の「テストが開発機に
左右される」罠を画面のテストが踏む。実際 `NotepadScreen` に数えさせた版では
keyboard 画面のテストが**このマシンに Vaporizer2 が入っているせいで**落ちた。
`NotepadScreenParts.catalog_notes` として app から渡す形に直してある。

### 番人テスト

| テスト | 場所 |
|---|---|
| `a_plugin_without_patch_dirs_is_reported_as_skipped` | `cmrt-runtime/src/core_config/tests.rs` |
| `a_plugin_whose_patch_dirs_all_vanished_is_reported_as_missing` | 同上 |
| `a_plugin_with_patch_dirs_is_not_reported_as_skipped` | 同上 |
| `the_default_plugin_is_never_reported_as_skipped` | 同上 |
| `the_report_lists_skipped_plugins_even_when_there_are_none` | `app/src/tui/patch_role_report/tests.rs` |
| `the_screens_show_which_plugins_are_missing_from_the_catalog` | `app/src/tui/tests/vaporizer2_screens.rs` |
| `the_screens_show_no_note_when_every_plugin_is_in_the_catalog` | 同上 |
| `the_patch_select_shows_why_a_plugin_is_missing_from_the_list` | `mml-overlay/src/ui/tests.rs` |
| `the_selector_carries_the_reason_a_plugin_is_missing_from_the_catalog` | `grid-sequencer/src/patch_selector/tests/overlay.rs` |
| `the_screen_shows_why_a_plugin_is_missing_from_the_catalog` | `keyboard/src/ui/tests.rs` |
| `patch_select_screen_shows_why_a_plugin_is_missing_from_the_catalog` | `notepad/src/ui/tests/overlay_screens/patch_select.rs` |
