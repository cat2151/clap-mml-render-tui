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
