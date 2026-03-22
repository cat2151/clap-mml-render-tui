use super::*;

#[test]
fn update_subcommand_is_recognized() {
    let args = vec!["cmrt".to_string(), "update".to_string()];

    assert!(is_update_subcommand(&args));
}

#[test]
fn update_subcommand_takes_precedence_over_cli_mml_mode() {
    let args = vec!["cmrt".to_string(), "update".to_string()];

    assert_eq!(cli_mml_arg(&args), None);
}

#[test]
fn cli_mml_mode_still_accepts_regular_positional_argument() {
    let args = vec!["cmrt".to_string(), "cde".to_string()];

    assert_eq!(cli_mml_arg(&args), Some("cde"));
    assert!(!is_update_subcommand(&args));
}
