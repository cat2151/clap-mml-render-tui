use super::*;

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('?'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Help));
}

#[test]
fn handle_normal_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = (0..8).map(|i| format!("line {i}")).collect();
    app.normal_page_size = 3;
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::PageDown);
    assert_eq!(app.cursor, 4);
    assert_eq!(app.list_state.selected(), Some(4));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 4"
    ));

    app.handle_normal(KeyCode::PageUp);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 1"
    ));
}

#[test]
fn handle_normal_j_prefetches_predicted_navigation_cache() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    app.normal_page_size = 2;

    app.handle_normal(KeyCode::Char('j'));

    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key("line 0"));
    assert!(cache.contains_key("line 2"));
    assert!(cache.contains_key("line 3"));
}

#[test]
fn handle_normal_f_shows_error_when_current_line_has_no_patch_json() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patch name JSON が見つかりません"
    ));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_p_shows_error_when_yank_buffer_is_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('p'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "yank バッファが空です"
    ));
    assert_eq!(app.lines, vec!["cde".to_string()]);
}

#[test]
fn handle_normal_f_enters_patch_phrase_for_current_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} cde"#.to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Pads/Pad 1.fxp"));
    assert_eq!(app.patch_phrase_history_items(), vec!["c".to_string()]);
    assert_eq!(app.patch_phrase_favorite_items(), vec!["c".to_string()]);
}

#[test]
fn handle_normal_o_and_o_insert_blank_line_and_enter_insert_mode() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('o'));

    assert_eq!(
        app.lines,
        vec!["line 0".to_string(), "line 1".to_string(), String::new()]
    );
    assert_eq!(app.cursor, 2);
    assert_eq!(app.list_state.selected(), Some(2));
    assert!(matches!(app.mode, Mode::Insert));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('O'));

    assert_eq!(
        app.lines,
        vec!["line 0".to_string(), String::new(), "line 1".to_string()]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(app.mode, Mode::Insert));
}

#[test]
fn handle_normal_w_launches_daw() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert!(matches!(app.mode, Mode::Normal));
}
