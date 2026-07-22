use super::*;

#[test]
fn handle_notepad_history_slash_then_enter_keeps_filtered_results_for_j_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "alpha".to_string(),
        "beta jk".to_string(),
        "gamma jk".to_string(),
    ];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('j'));
    app.handle_notepad_history(KeyCode::Char('k'));
    app.handle_notepad_history(KeyCode::Enter);
    app.handle_notepad_history(KeyCode::Char('j'));

    assert!(!app.notepad_history.filter_active);
    assert_eq!(app.notepad_history.query, "jk");
    assert_eq!(
        app.notepad_history_items(),
        vec!["beta jk".to_string(), "gamma jk".to_string()]
    );
    assert_eq!(app.notepad_history.history_cursor, 1);
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "gamma jk"
    ));
}

#[test]
fn handle_notepad_history_allows_slash_character_in_filter_query() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "alpha".to_string(),
        "dir/name".to_string(),
        "dir other".to_string(),
    ];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('n'));

    assert!(app.notepad_history.filter_active);
    assert_eq!(app.notepad_history.query, "/n");
    assert_eq!(app.notepad_history_items(), vec!["dir/name".to_string()]);
}

#[test]
fn handle_notepad_history_filter_space_updates_query_before_preview_shortcut() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta soft".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('b'));
    app.handle_notepad_history(KeyCode::Char('e'));
    app.handle_notepad_history(KeyCode::Char('t'));
    app.handle_notepad_history(KeyCode::Char('a'));
    let preview_before_space = app.playback.play_state.lock().unwrap().clone();

    app.handle_notepad_history(KeyCode::Char(' '));

    assert!(app.notepad_history.filter_active);
    assert_eq!(app.notepad_history.query, "beta ");
    assert_eq!(app.notepad_history_items(), vec!["beta soft".to_string()]);
    assert!(*app.playback.play_state.lock().unwrap() == preview_before_space);
}

#[test]
fn handle_notepad_history_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('p'));
    app.handle_notepad_history(KeyCode::Char('a'));
    app.handle_notepad_history(KeyCode::Char('d'));
    app.handle_notepad_history_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_notepad_history(KeyCode::Char('X'));

    assert!(app.notepad_history.filter_active);
    assert_eq!(app.notepad_history.query, "Xpad");
}
