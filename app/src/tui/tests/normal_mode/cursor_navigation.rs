use super::*;

#[test]
fn handle_normal_home_moves_to_first_line_and_plays_destination_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    app.editor.cursor = 3;
    app.editor.list_state.select(Some(3));

    app.handle_normal(KeyCode::Home);

    assert_eq!(app.editor.cursor, 0);
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 0"
    ));
}

#[test]
fn handle_normal_upper_l_no_longer_moves_to_last_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    let result = app.handle_normal(KeyCode::Char('L'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Idle
    ));
}

#[test]
fn handle_normal_shift_l_has_no_effect() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Idle
    ));

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
}
