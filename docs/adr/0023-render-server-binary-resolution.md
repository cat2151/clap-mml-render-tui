# ADR 0023: render-server の実体も play server と同じ順で探し、PATH を見ない

- 状態: 採用
- 関連: [0017](0017-play-server-binary-resolution.md) / [0010](0010-two-repo-layout.md)

## 決定

`offline_render_server_command` が空のとき、render-server の実体は
**「`cmrt.exe` と同じディレクトリ → 兄弟 repo `clap-mml-play-server` の `target/release/`」の順**で探す。
**PATH は見ない。** 見つからなければ、探した場所を並べた文言で render エラーにする
（起動を試みず、確定的に失敗させる）。

- 探索の本体は `cmrt-runtime::resolve_sibling_binary` / `not_found_lines`（[0017](0017-play-server-binary-resolution.md)
  で play server 用に作ったものと同じ関数）。呼び出し側は探す実行ファイル名（`clap-mml-render-server.exe`）
  だけを渡す
- 解決は `RenderServerSupervisor::new` で 1 度だけ行い、結果（`Result<ResolvedRenderServerCommand, String>`）を
  保持する。起こし直しても実体は変わらない（[0017](0017-play-server-binary-resolution.md) と同じ理由）
- 起動時ログに 1 行 `offline-render: render-server: exe=<path> (source=<同じディレクトリ|兄弟 repo の release|command>)`
  を出す

## play server と探索を共有する理由

render-server も play server も、同じ兄弟 repo（`clap-mml-play-server`）の `target/release/` に居る
実行ファイル 2 本という点で条件が同じ。探索を 2 か所に別々に書くと、[0017](0017-play-server-binary-resolution.md)
が潰した「PATH 上の古い実体が黙って選ばれる」事故が片方にだけ残る形で再発しうる
（実際に render-server 側だけ直っていなかった実測がある）。本体を 1 つの関数へ寄せ、
呼び出し側の違い（探す exe 名・掴んだ後に何をするか）だけを残した。

## PATH を見ない理由

[0017](0017-play-server-binary-resolution.md) と同じ。開発中は `cross_repo_local.py on` +
`cargo build --release` で兄弟 repo の最新が使われるべきで、PATH の exe を手で入れ替える運用は
想定していない。PATH を見る経路が残っていると、配布用に PATH へ通した古い exe が
兄弟 repo の最新ビルドより優先されうる。

## profile バッジ・staleness 判定を移さなかった理由

[0017](0017-play-server-binary-resolution.md) の `ServerProfile` 判定・`stale_source` 判定・
「debug のときは画面の文字列を変える」バッジは、DAW / TUI の画面に表示する場所があるから意味を持つ。
render-server はバックグラウンドで動く HTTP サーバーで、そういう画面が無い。ログに実体のパスを
1 行出すだけで、壊れていれば `render-server: exe=…` の行を見れば分かる。この非対称のぶん、
探索本体（`cmrt-runtime`）と呼び出し側（`realtime-play` / `offline-render`）の責務分担は
[0017](0017-play-server-binary-resolution.md) より薄い（`realtime-play/src/server_binary.rs` は
profile・staleness・`ServerSource`（Argument/ShellCommand）を引き続き持ち、`offline-render` は
持たない）。

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `cmrt-runtime` `sibling_binary::tests::the_executable_next_to_current_exe_is_used_when_it_exists` | 同じディレクトリの実体が優先されなくなった |
| `cmrt-runtime` `sibling_binary::tests::the_sibling_repo_release_is_used_when_nothing_sits_next_to_current_exe` | 兄弟 repo の release が選ばれなくなった |
| `cmrt-runtime` `sibling_binary::tests::neither_location_existing_reports_both_searched_places` | 見つからないときのエラーに探した場所が並ばなくなった |
| `cmrt-runtime` `sibling_binary::tests::the_repo_release_path_only_applies_to_a_cargo_build_layout` | 配布レイアウト（`target/debug`・`target/release` 以外）でも兄弟 repo を探しに行くようになった |
| `cmrt-offline-render` `render_server::command_resolution::tests::an_empty_command_finds_the_binary_next_to_current_exe` | render-server 側の呼び出しが探索本体を通らなくなった |
| `cmrt-offline-render` `render_server::command_resolution::tests::an_empty_command_with_nothing_found_reports_the_searched_places` | 見つからないときに render エラーへ探した場所が乗らなくなった |
| `cmrt-offline-render` `render_server::command_resolution::tests::an_explicit_command_is_resolved_as_shell_without_touching_current_exe` | render-server 起動コマンドを config で明示したときに探索へ落ちるようになった（PATH 解決の復活を含む） |
| `cmrt-realtime-play` `server_binary::tests`（既存、無改造） | play server 側の探索が `cmrt-runtime` への委譲後も壊れていないことの保険 |
