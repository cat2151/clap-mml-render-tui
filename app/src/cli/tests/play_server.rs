use super::*;

/// `--play-server` はモードを変えず、どのモードからも同じ意味で使える。
/// 「実体を指定して起動する」は TUI でも `render-mml` でも同じことなので。
#[test]
fn play_server_can_be_given_in_any_mode() {
    let invocation =
        parse_cli_invocation_from(["cmrt", "--play-server", "X:/x/server.exe"]).unwrap();
    assert_eq!(invocation.action, CliAction::Tui);
    assert_eq!(
        invocation.play_server,
        Some(PathBuf::from("X:/x/server.exe"))
    );

    let with_subcommand = parse_cli_invocation_from([
        "cmrt",
        "render-mml",
        "--play-server",
        "X:/x/server.exe",
        "cde",
    ])
    .unwrap();
    assert_eq!(
        with_subcommand.play_server,
        Some(PathBuf::from("X:/x/server.exe"))
    );
    assert!(matches!(with_subcommand.action, CliAction::RenderMml(_)));
}

#[test]
fn no_play_server_argument_leaves_the_search_to_decide() {
    assert_eq!(
        parse_cli_invocation_from(["cmrt"]).unwrap().play_server,
        None
    );
}

/// 打った指定が黙って無視されるのは、ADR 0017 が潰した事故と同じ手触りになる。
/// 存在しないなら探索へ落とさず、その場で止める。
#[test]
fn a_play_server_path_that_does_not_exist_is_an_error() {
    let error = play_server_launch(PathBuf::from("X:/no/such/server.exe")).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("--play-server"), "{message}");
    assert!(message.contains("server.exe"), "{message}");
}

#[test]
fn an_existing_play_server_path_becomes_an_explicit_executable() {
    let path = std::env::current_exe().expect("テストバイナリ自身は必ず存在する");

    let launch = play_server_launch(path.clone()).unwrap();

    assert_eq!(launch, cmrt_runtime::PlayServerLaunch::Executable(path));
}
