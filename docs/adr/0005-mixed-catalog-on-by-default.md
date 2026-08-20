# ADR 0005: 混在カタログは既定 ON。実在する dir だけ載せる

- 状態: 採用（2026-08-20）
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

## 罠: テストが開発機に左右される

カタログが「このマシンに何がインストールされているか」に依存するので、
**`Config` を作って `catalog_plugins` / `PatchPlugins::from_config` / `collect_patch_pairs` を
呼ぶテストを新しく書かないこと。カタログを手で並べて渡すこと。**

実際にこれで 9 本落ちた（`daw` の random patch 系がフィクスチャの代わりに実物の音色を掴んだ）。
外から渡せる口を用意してある:

- `catalog_plugins_with(cfg, installed)`（`cmrt-runtime`）
- `SourceSet::from_catalog(cfg, &PatchPlugins)`（`app/src/voicing_sources.rs`）
- `VoicingPolicies { plugins }` を直接組むテストヘルパ（`app/src/tui/voicing/tests.rs`）

## 実測（2026-08-20 / release / `active_plugin = 'Dexed'`）

`cmrt patch-roles`: カタログは Dexed（既定）→ Surge XT の 2 つ、Dexed は 1 度しか出ない。
patch 件数 **4064**（= Surge 3008 + Dexed 1056）。
候補数 Chord 1807 / Bass 1464 / Arpeggio 2059 / Free 3313 /
Kick 1106 / Snare 1101 / HiHat 1078 / Percussion 1178。
→ ベースラインは [0011](0011-verification-and-baselines.md)
