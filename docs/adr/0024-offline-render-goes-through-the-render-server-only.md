# ADR 0024: offline render は render-server の 1 経路だけ。`cmrt.exe` は CLAP をロードしない

- 状態: 採用
- 関連: [0009](0009-offline-entry-map.md)（本 ADR で廃止） / [0022](0022-effect-chain-in-init-json.md) /
  [0023](0023-render-server-binary-resolution.md)

## 決定

offline render（notepad の WAV、DAW の cell cache、`render-mml`、`--server`、CLI モード）は
**すべて `OfflineRenderer` → render-server（`POST /render`）の 1 経路**を通す。
TUI プロセス内で CLAP をロードする `in_process` backend と、それを選ぶ config キー
`offline_render_backend` は廃止する。**`cmrt.exe` は CLAP plugin を一切ロードしない。**

- `OfflineRenderer::new(cfg)`。plugin entry の表を受け取らない
- `offline_render_backend` / `offline_render_workers` は config に残っていても**読み飛ばす**
  （エラーにしない。[0017](0017-play-server-binary-resolution.md) の `realtime_play_server_command` と同じ扱い）
- `cmrt … --config <path>` で読んだ config は、子 render-server にも `--config <path>` として渡す
  （`Config::source_path`。toml からは設定できず、`--config` 分岐だけが埋める）。
  `offline_render_server_command` で起動コマンドを明示したときは渡さない
- render-server の実体の探し方は [0023](0023-render-server-binary-resolution.md) のまま

## 理由

backend が 2 つあると、**機能が片方にしか付かない**。effect chain（[0022](0022-effect-chain-in-init-json.md)）が
先に `in_process` にだけ載り、`render_server` を使っている環境では chain 付きの cell が HTTP 500 になった。
「黙って dry にしない」設計なので壊れ方としては正しいが、**動く経路がユーザーの使っている経路ではない**
という形で現れる。経路を 1 本にすれば、この種の不一致は原理的に起きない。

`--server` と CLI モードも同じ理由で render-server 経路へ寄せた。この 2 つは offline render と
plugin entry を共有していたので、残すと entry のロードと MML → plugin の引き分けが TUI 側に残り、
3 本目の経路になる。

## 却下した案

- **`in_process` を test-only の経路として残す。** TUI 側の render 関数は wrapper（probe 付き render・
  prepare queue）で、test 目的で残すとそれが全部残る。同じ主張のテストは play-server 側
  （`cmrt-core` の `pipeline::tests::effects`）が持っている
- **`offline_render_backend = "in_process"` をエラー停止にする**（`realtime_audio_backend = "in_process"` の先例）。
  生成された config は `in_process` を明示で書き出していたので、全員が起動時に止まる。あちらは乗り換え先が
  2 つあって選ばせる意味があったが、こちらは 1 つしか無く、直し方も「行を消す」だけ
- **`--config` を子へ渡さない。** `in_process` があるうちは `--config` で `[plugins]` を試せたが、廃止後は
  子へ渡さないと `--config` の `[plugins]` / port が render に効かず、[0011](0011-verification-and-baselines.md) の
  「実ユーザーの config に触らずに試す」が `render-mml` で成立しなくなる

## 結果

- TUI の workspace は `clack-host` に直接依存しない（`cargo tree -i clack-host` の直接依存元は play-server 側の crate だけ）
- `offline_render_server_workers` / `offline_render_server_port` / `offline_render_server_command` の 3 キーだけが
  offline render の設定。notepad / DAW の render worker 数は `offline_render_server_workers` で決まる
- effect の catalog（DAW の `x` overlay）は TUI 内の `EffectPlugins::discover()`（preset ファイルの走査。DLL はロードしない）で取る。
  TUI と render-server で catalog がずれうる論点は未解決
- `render-mml` の `patch_name` は `(render-server)` 固定になる（引き分けは render-server 側）

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `cmrt-runtime` `tests::the_removed_offline_render_backend_keys_are_ignored` | 古い config の 2 キーで起動が止まるようになった |
| `cmrt-offline-render` `render_server::command_resolution::tests::a_found_binary_forwards_the_config_path_as_an_argument` | `--config` が子 render-server に届かなくなった |
| `cmrt-offline-render` `render_server::command_resolution::tests::an_explicit_shell_command_is_not_given_the_config_path` | 明示した起動コマンドに `--config` が足されるようになった |
| play-server `cmrt-core` `pipeline::tests::effects::unsupported_route_rejects_a_chain_before_rendering` | effect を持たない経路が chain を黙って dry で通すようになった |
