# ADR 0010: 2 repo 構成は TUI → play-server の一方向。cross-repo はローカルモードで回す

- 状態: 採用（2026-08-20）
- 関連: [0007](0007-patch-role-defaults-three-layers.md)

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
  `package collision in the lockfile` で止まる。**本当の原因はバージョンではなく名前の重複**だった
  （過去は version を上げる回避策を取っていた）。TUI 側 core-lib を `cmrt-render-core` へ改名して解消。
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

```
Adding cmrt-core          v0.1.0 (..\clap-mml-play-server\core-lib)
Adding cmrt-server-config v0.1.0 (..\clap-mml-play-server\server-config)
```

## 罠

- **`cross_repo_local_off.bat` は `git checkout -- Cargo.lock` をする。** ローカルモード中に行った
  正当な `cargo update` の結果も、未 commit の lock 変更も巻き戻る。ローカルモード中の lock は
  `[patch]` で `source` 行が剥がれた別物なので「一部だけ残す」ことはできない。
  **lock を触る作業とローカル横断モードを混ぜないこと**
- **ローカルモード ON 中の `Cargo.lock` は commit してはいけない**
- **bat は CRLF 必須。** LF だと cmd がパースに失敗して全行がコマンド扱いになる
  （`'server"' is not recognized as an internal or external command`）。
  worktree を CRLF へ直しても `git diff` は差分なし扱いで、次に git が触ると LF へ戻るので、
  **`.gitattributes` に `*.bat text eol=crlf` が要る**。日本語を出すので先頭に `chcp 65001 > nul`
- `.cargo/config.toml` の `[patch]` の**相対パスは config ファイルの位置基準**で解決される
- **`ServerConfig::load()` はひな形を作らない。** config.toml が無い環境でサーバーを単体起動すると
  エラーになる（ひな形は TUI 固有項目まで含むので TUI の責務）
- **nightly workflow `update-cat2151-rust-deps.yml`（JST 01:00）が git 依存の lock を自動追従して
  commit する。**「push した直後は TUI が壊れている」状態を放置すると翌朝 CI が赤くなる
- **実機で動かすには play server のバイナリが PATH 上に要る。** `for_local.bat` が
  `./target/debug` と `../clap-mml-play-server/target/debug` を PATH に載せる。
  play-server を変更したら**必ず `../clap-mml-play-server` のデバッグビルドを行うこと**
- **`SURGE_XT_PLUGIN_ID` が play-server repo 内 2 か所にある**
  （`server-config/src/plugin_identity.rs` と `core-lib/src/surge_data.rs`）。統合するなら
  `core-lib` → `server-config` の依存を足す形になるので**未着手**
