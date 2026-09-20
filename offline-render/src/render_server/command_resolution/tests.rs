use super::*;

#[test]
fn an_explicit_command_is_resolved_as_shell_without_touching_current_exe() {
    let resolved = resolve_render_server_command_with("exit 0", None).unwrap();

    assert_eq!(resolved.source_label(), "command");
    assert_eq!(resolved.describe(), "exit 0");
}

/// `offline_render_server_command` が空で、探索も何も見つけられなければ
/// render エラー（PATH へは落ちない）。
#[test]
fn an_empty_command_with_nothing_found_reports_the_searched_places() {
    let unresolvable = std::env::temp_dir()
        .join(format!(
            "cmrt_render_server_test_no_such_dir_{}",
            std::process::id()
        ))
        .join("cmrt.exe");

    let message = resolve_render_server_command_with("", Some(&unresolvable)).unwrap_err();

    assert!(message.contains("render-server"), "{message}");
    assert!(message.contains("探した場所"), "{message}");
}

/// `offline_render_server_command` が空でも、`cmrt.exe` と同じディレクトリに
/// render-server の実体があればそれを使う。
#[test]
fn an_empty_command_finds_the_binary_next_to_current_exe() {
    let root = std::env::temp_dir().join(format!(
        "cmrt_render_server_test_sibling_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let cmrt = root.join("cmrt.exe");
    let server = root.join(default_render_server_executable_name());
    std::fs::write(&server, []).unwrap();

    let resolved = resolve_render_server_command_with("", Some(&cmrt)).unwrap();

    assert_eq!(resolved.source_label(), "同じディレクトリ");
    assert_eq!(resolved.describe(), server.display().to_string());

    let _ = std::fs::remove_dir_all(&root);
}

fn command_args(command: &Command) -> Vec<String> {
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_found_binary_forwards_the_config_path_as_an_argument() {
    let resolved = ResolvedRenderServerCommand::Binary {
        path: PathBuf::from("render-server.exe"),
        source: SiblingBinarySource::SiblingDirectory,
    };
    let config_path = PathBuf::from("alt-config.toml");

    let command = resolved.build_command(Some(&config_path));

    assert_eq!(
        command_args(&command),
        vec!["--config".to_string(), "alt-config.toml".to_string()]
    );
}

#[test]
fn a_found_binary_gets_no_arguments_without_a_config_path() {
    let resolved = ResolvedRenderServerCommand::Binary {
        path: PathBuf::from("render-server.exe"),
        source: SiblingBinarySource::SiblingRepoRelease,
    };

    let command = resolved.build_command(None);

    assert!(command_args(&command).is_empty());
}

/// 明示の shell command は書かれたとおりに起動する。`--config` を後ろへ足すと
/// `cmd /C "<command> --config <path>"` になり、command の書き方次第で意味が変わる。
#[test]
fn an_explicit_shell_command_is_not_given_the_config_path() {
    let resolved = ResolvedRenderServerCommand::Shell("exit 0".to_string());
    let config_path = PathBuf::from("alt-config.toml");

    let command = resolved.build_command(Some(&config_path));

    let args = command_args(&command);
    assert!(!args.iter().any(|arg| arg == "--config"), "{args:?}");
    assert_eq!(args.last().map(String::as_str), Some("exit 0"));
}
