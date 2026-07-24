use super::*;

#[test]
fn handle_normal_page_down_and_page_up_move_by_visible_page() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = (0..8).map(|i| format!("line {i}")).collect();
    app.editor.page_size = 3;
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    app.handle_normal(KeyCode::PageDown);
    assert_eq!(app.editor.cursor, 4);
    assert_eq!(app.editor.list_state.selected(), Some(4));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "line 4"
    ));

    app.handle_normal(KeyCode::PageUp);
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "line 1"
    ));
}

#[test]
fn handle_normal_j_prefetches_predicted_navigation_cache() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    app.editor.page_size = 2;

    app.handle_normal(KeyCode::Char('j'));

    let cache = app.audio.cache.lock().unwrap();
    assert!(cache.contains_key("line 0"));
    assert!(cache.contains_key("line 2"));
    assert!(cache.contains_key("line 3"));
}

#[test]
fn handle_normal_j_prefetches_direction_first_then_fills_remaining_navigation_targets() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = (0..12).map(|i| format!("line {i}")).collect();
    app.editor.cursor = 4;
    app.editor.page_size = 5;
    app.editor.list_state.select(Some(4));

    app.handle_normal(KeyCode::Char('j'));

    assert_eq!(
        app.audio
            .order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["line 6", "line 7", "line 4", "line 10", "line 0", "line 8", "line 9",]
    );
}
