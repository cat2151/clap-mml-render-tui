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

# その他
- デバウンス禁止
- cat2151のライブラリは、「revision固定を禁止。さらに、古い lock を放置せず最新 HEAD へ追従すること」
- issue-notes/は更新を禁止
- README.mdは更新禁止。README.ja.mdから生成されるので。

# 完了時
- 450行をoverした*.rsは、単一責任の原則に従いファイル分割
- cargoのclippyとfmtを使うこと
- リリースビルド（ cargo build --release ）をすること
- プルリクエストは日本語で書くこと
