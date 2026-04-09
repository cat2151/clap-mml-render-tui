use super::*;

#[test]
fn update_subcommand_is_recognized() {
    assert_eq!(
        parse_cli_from(["cmrt", "update"]).unwrap(),
        CliAction::Update
    );
}

#[test]
fn update_subcommand_takes_precedence_over_cli_mml_mode() {
    assert_ne!(
        parse_cli_from(["cmrt", "update"]).unwrap(),
        CliAction::CliMml("update".to_string())
    );
}

#[test]
fn cli_mml_mode_still_accepts_regular_positional_argument() {
    assert_eq!(
        parse_cli_from(["cmrt", "cde"]).unwrap(),
        CliAction::CliMml("cde".to_string())
    );
}

#[test]
fn server_flag_uses_default_port_when_value_is_omitted() {
    assert_eq!(
        parse_cli_from(["cmrt", "--server"]).unwrap(),
        CliAction::Server(server::DEFAULT_PORT)
    );
}

#[test]
fn shutdown_flag_uses_default_port_when_value_is_omitted() {
    assert_eq!(
        parse_cli_from(["cmrt", "--shutdown"]).unwrap(),
        CliAction::Shutdown(server::DEFAULT_PORT)
    );
}

#[test]
fn deprecated_mml_flag_returns_guidance() {
    let err = parse_cli_from(["cmrt", "--mml", "cde"]).unwrap_err();

    assert!(err
        .to_string()
        .contains("`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。"));
}

#[test]
fn deprecated_mml_flag_without_value_returns_same_guidance() {
    let err = parse_cli_from(["cmrt", "--mml"]).unwrap_err();

    assert!(err
        .to_string()
        .contains("`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。"));
}

#[test]
fn top_level_help_uses_runtime_default_port() {
    match parse_cli_from(["cmrt", "--help"]).unwrap() {
        CliAction::Help(help) => assert!(help.contains(&format!(
            "curl -X POST http://127.0.0.1:{}/ --data 'cde'",
            server::DEFAULT_PORT
        ))),
        other => panic!("expected help action, got {other:?}"),
    }
}

#[test]
fn subcommand_help_is_preserved() {
    match parse_cli_from(["cmrt", "update", "--help"]).unwrap() {
        CliAction::Help(help) => {
            assert!(help.contains("Usage: cmrt update"));
            assert!(help.contains("アップデートを実行"));
        }
        other => panic!("expected help action, got {other:?}"),
    }
}
