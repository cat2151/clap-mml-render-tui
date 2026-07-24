use super::*;

#[test]
fn handle_notepad_history_j_prefetches_predicted_navigation_cache() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "zero".to_string(),
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
    ];
    app.notepad_history.page_size = 2;
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('j'));

    let cache = app.audio.cache.lock().unwrap();
    assert!(cache.contains_key("zero"));
    assert!(cache.contains_key("two"));
    assert!(cache.contains_key("three"));
}

#[test]
fn handle_notepad_history_j_prefetches_direction_first_then_fills_remaining_navigation_targets() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = (0..12).map(|i| format!("history {i}")).collect();
    app.notepad_history.page_size = 5;
    app.start_notepad_history();
    app.notepad_history.history_cursor = 4;
    app.notepad_history.history_state.select(Some(4));

    app.handle_notepad_history(KeyCode::Char('j'));

    assert_eq!(
        app.audio
            .order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "history 6",
            "history 7",
            "history 4",
            "history 10",
            "history 0",
            "history 8",
            "history 9",
        ]
    );
}

#[test]
fn handle_notepad_history_k_prefetches_page_up_before_page_down_then_far_direction_targets() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = (0..12).map(|i| format!("history {i}")).collect();
    app.notepad_history.page_size = 5;
    app.start_notepad_history();
    app.notepad_history.history_cursor = 6;
    app.notepad_history.history_state.select(Some(6));

    app.handle_notepad_history(KeyCode::Char('k'));

    assert_eq!(
        app.audio
            .order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "history 4",
            "history 3",
            "history 6",
            "history 0",
            "history 10",
            "history 2",
            "history 1",
        ]
    );
}
