# Agent setup

Before running Rust checks in this repository, from the repository root prepare the test environment:

```bash
./scripts/setup-cargo-test-env.sh
```

After setup, validate from the repository root with:

```bash
cargo test
```

The setup script installs the Linux packages needed for this workspace's `rodio`/`cpal` dependency chain to build `alsa-sys`.

# その他
- プルリクエストは日本語で書くこと
- cargoのformatとlinterを使うこと
- デバウンス禁止
- cat2151のライブラリはrevision固定を禁止
- issue-notes/は更新を禁止
