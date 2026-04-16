use super::*;

#[test]
fn clamp_session_cursor_caps_to_last_available_line() {
    assert_eq!(crate::tui::session::clamp_session_cursor(0, 1), 0);
    assert_eq!(crate::tui::session::clamp_session_cursor(2, 3), 2);
    assert_eq!(crate::tui::session::clamp_session_cursor(9, 3), 2);
}

#[test]
fn begin_playback_session_invalidates_previous_session() {
    let app = TuiApp::new_for_test(test_config());

    let first = app.begin_playback_session();
    let second = app.begin_playback_session();

    assert!(!app.test_is_current_playback_session(first));
    assert!(app.test_is_current_playback_session(second));
}

#[test]
fn set_play_state_if_current_ignores_stale_session() {
    let app = TuiApp::new_for_test(test_config());

    let stale = app.begin_playback_session();
    let current = app.begin_playback_session();
    let newer = app.begin_playback_session();

    app.set_play_state_if_current(stale, PlayState::Done("old".to_string()));
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));

    app.set_play_state_if_current(current, PlayState::Running("new".to_string()));
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));

    app.set_play_state_if_current(newer, PlayState::Running("new".to_string()));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "new"
    ));
}

#[test]
fn save_history_state_persists_tui_cursor_lines_and_mode_flag() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_save_history_state_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string(), "def".to_string(), "ghi".to_string()];
    app.cursor = 2;
    app.is_daw_mode = true;

    app.save_history_state();

    let history_path = crate::test_utils::session_state_path_for_test()
        .expect("config local dir should resolve in isolated TUI history test");
    assert!(
        history_path.exists(),
        "expected isolated history file to be created at {}",
        history_path.display()
    );
    let saved = crate::history::load_session_state();
    assert_eq!(saved.cursor, 2);
    assert_eq!(saved.lines, app.lines);
    assert!(saved.is_daw_mode);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn daw_mode_switch_request_can_be_consumed_from_tui_runtime() {
    assert!(!crate::daw::take_http_mode_switch_request());

    crate::daw::request_http_mode_switch();

    assert!(crate::daw::take_http_mode_switch_request());
    assert!(!crate::daw::take_http_mode_switch_request());
}
