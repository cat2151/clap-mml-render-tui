use super::*;

#[test]
fn handle_normal_dd_yanks_current_line_and_keeps_notepad_non_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    let result = app.handle_normal(KeyCode::Char('d'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(app.normal_pending_delete);
    assert!(app.yank_buffer.is_none());

    let result = app.handle_normal(KeyCode::Char('d'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.lines, vec!["line 0".to_string(), "line 2".to_string()]);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(!app.normal_pending_delete);
    assert_eq!(app.yank_buffer.as_deref(), Some("line 1"));
}

#[test]
fn handle_normal_d_is_cleared_when_another_key_is_pressed() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 0;
    app.list_state.select(Some(0));

    app.handle_normal(KeyCode::Char('d'));
    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert_eq!(app.lines, vec!["line 0".to_string(), "line 1".to_string()]);
    assert!(!app.normal_pending_delete);
    assert!(app.yank_buffer.is_none());
}

#[test]
fn handle_normal_dd_on_single_line_replaces_with_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["only".to_string()];

    app.handle_normal(KeyCode::Char('d'));
    app.handle_normal(KeyCode::Char('d'));

    assert_eq!(app.lines, vec![String::new()]);
    assert_eq!(app.cursor, 0);
    assert_eq!(app.yank_buffer.as_deref(), Some("only"));
}

#[test]
fn handle_normal_delete_yanks_current_line_and_keeps_notepad_non_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Delete);

    assert_eq!(app.lines, vec!["line 0".to_string(), "line 2".to_string()]);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert_eq!(app.yank_buffer.as_deref(), Some("line 1"));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["only".to_string()];

    app.handle_normal(KeyCode::Delete);

    assert_eq!(app.lines, vec![String::new()]);
    assert_eq!(app.cursor, 0);
    assert_eq!(app.yank_buffer.as_deref(), Some("only"));
}

#[test]
fn handle_normal_p_and_p_paste_yanked_line_below_or_above_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 0;
    app.list_state.select(Some(0));
    app.yank_buffer = Some("yanked".to_string());

    app.handle_normal(KeyCode::Char('p'));

    assert_eq!(
        app.lines,
        vec![
            "line 0".to_string(),
            "yanked".to_string(),
            "line 1".to_string()
        ]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));
    app.yank_buffer = Some("yanked".to_string());

    app.handle_normal(KeyCode::Char('P'));

    assert_eq!(
        app.lines,
        vec![
            "line 0".to_string(),
            "yanked".to_string(),
            "line 1".to_string()
        ]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
}

#[test]
fn handle_normal_home_and_l_move_to_edges_and_play_destination_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('L'));

    assert_eq!(app.cursor, 3);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 3"
    ));

    app.handle_normal(KeyCode::Home);

    assert_eq!(app.cursor, 0);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 0"
    ));
}

#[test]
fn handle_normal_shift_l_toggles_random_log_pane_without_moving_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert!(app.notepad_random_log.visible);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));

    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));

    assert!(!app.notepad_random_log.visible);
    assert_eq!(app.cursor, 1);
}
