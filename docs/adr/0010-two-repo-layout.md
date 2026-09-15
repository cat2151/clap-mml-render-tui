# ADR 0010: 2 repo 構成は TUI → play-server の一方向。cross-repo はローカルモードで回す

- 状態: 採用
- 関連: [0007](0007-patch-role-defaults-three-layers.md) / [0017](0017-play-server-binary-resolution.md)

## 決定

crate の依存の辺は**すべて TUI → play-server の一方向**。逆向きの辺は作らない。

```
[ clap-mml-play-server ]
  server-config (package: cmrt-server-config)   ← 葉 crate。CLAP も cmrt-core も引かない
    ├ ServerConfig            … サーバーが読む config.toml の項目だけ
    ├ PluginProfile / PatchRoleFilters / builtin_plugin_profiles
    ├ plugin_defaults          … OS ごとの Surge / Dexed 標準インストール先・音色置き場
    ├ plugin_identity          … plugin_file_stem / SURGE_XT_PLUGIN_ID / DEXED_PLUGIN_ID
    ├ paths                    … config_app_dir / config_file_path
    └ patch_dirs               … configured_patch_dirs / patch_root_dir / shared_patch_root_dir
        ▲ path                              ▲ path
  render-server               realtime-play-server        ← TUI への依存は無い

[ clap-mml-render-tui ]
  cmrt-runtime ──git──► cmrt-server-config
    └ Config … TUI が読む全項目。plugin 解決は上の crate へ委譲
  core-lib (package: cmrt-render-core、lib name は cmrt_core のまま) ──git──► play-server の cmrt-core
```

## push 往復が要っていた原因（3 つ別物）

- **原因 A: package 名 `cmrt-core` の重複。** `[patch]` で兄弟 repo をローカル参照すると
  `package collision in the lockfile` で止まる。**本当の原因はバージョンではなく名前の重複。**
  TUI 側 core-lib を `cmrt-render-core` へ改名して解消。
  **`[lib] name = "cmrt_core"` は据え置いたので `.rs` の変更は 1 行も要らなかった**
- **原因 B: 依存の向きが相互。** `play-server の servers → TUI の cmrt-runtime` が唯一の逆向き辺。
  `cmrt-server-config` を新設して解消
- **原因 C: 別 repo の型を構造体リテラルで組んでいる。** `cmrt_core::CoreConfig { .. }` を
  リテラルで組んでいたため、原因 B と組み合わさって**play-server に 1 フィールド足した瞬間に
  play-server 自身がビルド不能**になっていた

`CoreConfig` に `Default` を derive してあるのはこの再発を防ぐため
（テストのリテラルを `..Default::default()` で済ませ、フィールド追加で別 repo のテストが壊れないように）。
**本番のリテラルでは省略しない**決まり。

## 採らなかった設計

- **案 2b「サーバーへ設定を注入する（DI）」**: 依存は消えるが、**サーバー単体起動
  （`for_local.bat` から手で立ててログを見る運用）ができなくなる**か CLI 引数が長くなる
- **`plugin_identity` へ `is_surge_xt` 相当を入れる**: サーバー自身の Surge 判定は既存の
  `cmrt_core::plugin_is_surge`（ファイル名 fallback 付き）が担当していて判定規則が別物なので、
  3 つ目の実装を作らない

## cross-repo ローカルモードの運用

`cross_repo_local_on.bat` が `.cargo\config.toml` を生成し、TUI の git 依存 2 本を
兄弟 repo の**作業ツリー**（未 commit を含む）へ向ける。
**push を待たずに実装も検証も通る。push が要るのは TUI を commit する瞬間だけ。**

**ロジックの実体は `scripts/cross_repo_local.py`。`.bat` は Windows 用の入口でしかない。**
`on` / `off` に加えて `cross_repo_local_status.bat` があり、**commit して安全でなければ
非 0 で終了する**（後述の 3 つの壊れ方をすべて機械的に検出する）。
`[patch]` へ書く crate の表は Python 側 `PATCHED_CRATES` の 1 か所だけ。

### 人手に頼らない：pre-commit hook

`cross_repo_local_hooks.bat` が `core.hooksPath` を `.githooks` へ向け、`.githooks/pre-commit` が
`status --no-fetch --staged --fix` を走らせる。ローカルモードが ON なら play-server の作業ツリーが
clean かつ HEAD が push 済みであることを確認し、**自動で off・Cargo.lock 更新・stage まで行う**。
安全を確認できなければ何も変更せず commit を止める。`git commit` の経路が
人間でもエージェントでも IDE でも同じ門を通る。`core.hooksPath` は clone ごとのローカル設定なので
**commit では配れない。clone したら一度実行が要る**（`on` は未設定を検出して警告する）。

- 判定対象は worktree ではなく **index（= commit に載る中身）**。だから `--staged` を付ける。
  これで「`cargo update` はしたが `git add Cargo.lock` を忘れた」という、
  従来 `status` では見えなかった壊れ方も止まる
- ローカルモード ON なら、commit 直前に上記の安全条件を満たした場合だけ自動で OFF にする。
  play-server が未 push、作業ツリーが dirty、または兄弟 repo が見つからない場合は commit を止める

### なぜ .bat ではなく Python か

- **`cargo xtask` は採れない。** このツールの仕事は「ビルド設定を直すこと」なので、
  ワークスペースがビルドできない状態で動かせなければ意味がない（鶏と卵）
- 判定に `Cargo.lock` のパースと 2 repo の rev 比較が要る。`tomllib` と `git rev-parse` で
  素直に書ける一方、cmd では書けない
- `.bat` は CRLF 必須（後述）という壊れ方の温床がある。`.py` にはそれが無い

## off が Cargo.lock を「戻して、さらに進める」理由

`off` は `git checkout -- Cargo.lock` の**後に必ず `cargo update -p cmrt-core -p cmrt-server-config`
を走らせる**。checkout だけで止めると、lock は「ローカルモードに入る前」の古い rev へ戻る。
その rev には兄弟 repo で今まさに足した API が無いので、
**ローカルモードでは通っていたコードが、OFF にした瞬間ビルド不能になる**。
AGENTS.md の「古い lock を放置せず最新 HEAD へ追従」はこの経路にも効く。

## 罠

- **`cross_repo_local_off.bat` は `Cargo.lock` を HEAD へ戻す。** ローカルモード中に行った
  正当な `cargo update` の結果も、未 commit の lock 変更も巻き戻る。ローカルモード中の lock は
  `[patch]` で `source` 行が剥がれた別物なので「一部だけ残す」ことはできない。
  **lock を触る作業とローカル横断モードを混ぜないこと**（どうしても残すなら `off --keep-lock`）
- **ローカルモード ON 中の `Cargo.lock` は commit してはいけない**
- **復元は `git checkout -- <path>` ではなく `git restore --source=HEAD --staged --worktree` でなければならない。**
  前者が復元するのは HEAD ではなく **index**。ローカルモード中に `git add -A` していると、
  壊れた lock をそのまま書き戻したうえ index も壊れたまま残り、「HEAD の内容へ戻しました」と
  成功表示したまま直後の `cargo update` が
  `package ID specification 'cmrt-core' did not match any packages` で落ちる。
  同じ理由で差分表示も `git diff HEAD --` でないと staged 時に空になる。
  `status` が worktree だけでなく **index の lock** も見るのはこの事故を検出するため
- **CI は main への直 push をビルドしない。** `call-rust-windows-cargo-check.yml` は
  `pull_request: types: [closed]` でしか動かない。直 push で壊れた lock を入れると、
  気づくのは翌朝の nightly workflow になる。**commit 前に `cross_repo_local_status.bat`**
- **bat は CRLF 必須。** LF だと cmd がパースに失敗して全行がコマンド扱いになる
  （`'server"' is not recognized as an internal or external command`）。
  worktree を CRLF へ直しても `git diff` は差分なし扱いで、次に git が触ると LF へ戻るので、
  **`.gitattributes` に `*.bat text eol=crlf` が要る**。日本語を出すので先頭に `chcp 65001 > nul`
- `.cargo/config.toml` の `[patch]` の**相対パスは config ファイルの位置基準**で解決される
- **`ServerConfig::load()` はひな形を作らない。** config.toml が無い環境でサーバーを単体起動すると
  エラーになる（ひな形は TUI 固有項目まで含むので TUI の責務）
- **nightly workflow `update-cat2151-rust-deps.yml`（JST 01:00）が git 依存の lock を自動追従して
  commit する。**「push した直後は TUI が壊れている」状態を放置すると翌朝 CI が赤くなる
- play server の実体（バイナリ）をどう見つけるかは [0017](0017-play-server-binary-resolution.md)。
  play-server を変更したら**必ず `../clap-mml-play-server` の release ビルドを行うこと**
- **`SURGE_XT_PLUGIN_ID` が play-server repo 内 2 か所にある**
  （`server-config/src/plugin_identity.rs` と `core-lib/src/surge_data.rs`）。統合するなら
  `core-lib` → `server-config` の依存を足す形になるので**未着手**
