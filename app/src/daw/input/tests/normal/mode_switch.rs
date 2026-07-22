use super::*;

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('?'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Help));
    assert!(matches!(app.help_origin, DawMode::Normal));
}

#[test]
fn handle_normal_n_returns_to_tui() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('n'));

    assert!(matches!(result, super::super::DawNormalAction::ReturnToTui));
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_normal_v_launches_keyboard() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(KeyCode::Char('v'));

    assert!(matches!(
        result,
        super::super::DawNormalAction::LaunchKeyboard
    ));
}

#[test]
fn handle_normal_e_requests_config_edit() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('e'));

    assert!(matches!(result, super::super::DawNormalAction::EditConfig));
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_normal_esc_has_no_effect() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;

    let result = app.handle_normal(crossterm::event::KeyCode::Esc);

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.editor.cursor_track, 2);
    assert_eq!(app.editor.cursor_measure, 1);
}
