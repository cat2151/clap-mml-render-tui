# test

Before running Rust checks in this repository, from the repository root prepare the test environment:

```bash
./scripts/setup-cargo-test-env.sh
```

※Windows上では setup-cargo-test-env.sh は実行しないこと

After setup, validate from the repository root with:

```bash
cargo test
```

The setup script installs the Linux packages needed for this workspace's `rodio`/`cpal` dependency chain to build `alsa-sys`.

# モジュール/テスト配置規約
- `#[path]` 属性の使用を禁止。モジュールはRust標準のパス解決（`foo.rs` + `foo/` ディレクトリ）だけで構成すること
- テストは同居型に統一。モジュール `foo.rs` のテストは `foo/tests.rs` に置き、`#[cfg(test)] mod tests;` で宣言する
- テストが大きい場合は `foo/tests.rs` から `foo/tests/*.rs` へサブモジュール分割する
- `src/tests/` のような、実装と別階層のミラーツリーにテストを置くことを禁止
- ファイル分割時は責務ベースで命名すること。`help_and_mixer.rs` のような "X_and_Y" 接続詞命名を新規に作らない
- テストサブモジュールの下にさらにサブモジュールを掘る「分割の分割」を避け、親のtestsディレクトリ直下へフラットに置く

# その他
- デバウンス禁止
- cat2151のライブラリは、「revision固定を禁止。さらに、古い lock を放置せず最新 HEAD へ追従すること」
- issue-notes/は更新を禁止
- README.mdは更新禁止。README.ja.mdから生成されるので。
- ./target/release に clap-mml-realtime-play-server.exe をアドホックにcpすることを禁止（根が深いトラブルの温床になった）。
- 意図しない実行中serverを検出したらtaskkillすること
- ビルドロック回避を含め、`--target-dir`等で既定以外のtargetディレクトリを作成・使用することを禁止

# 完了時
- 450行をoverした*.rsは、単一責任の原則に従いファイル分割
- cargoのclippyとfmtを使うこと
- リリースビルド（ cargo build --release ）をすること
  - もし local ../clap-mml-play-server/ をメンテした場合、 ../clap-mml-play-server/ のリリースビルドをすること
  - いずれも人間が動作確認する用（debugビルドだと音が途切れて「バグか？」となったのでリリースビルドで動作確認する）
- プルリクエストは日本語で書くこと
