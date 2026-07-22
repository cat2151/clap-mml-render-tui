use super::*;

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('?'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Help));
}

#[test]
fn handle_normal_w_launches_daw() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_v_launches_keyboard() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('v'));

    assert!(matches!(result, NormalAction::LaunchKeyboard));
}

#[test]
fn handle_normal_b_requests_loop_browser_without_running_startup_inline() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('b'));

    assert!(matches!(result, NormalAction::LaunchLoopBrowser));
    assert!(matches!(app.mode, Mode::Normal));
    assert!(!app.loop_browser.state.starting);

    app.begin_loop_browser_startup();
    assert!(matches!(app.mode, Mode::LoopBrowser));
    assert!(app.loop_browser.state.starting);

    app.complete_loop_browser_startup();
    assert!(!app.loop_browser.state.starting);
    assert!(app
        .loop_browser
        .state
        .error
        .as_deref()
        .is_some_and(|error| error.contains("cmrt scan-loops")));
}

#[test]
fn handle_normal_e_requests_config_edit() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('e'));

    assert!(matches!(result, NormalAction::EditConfig));
    assert!(matches!(app.mode, Mode::Normal));
}
