use super::*;

#[test]
fn handle_notepad_history_j_previews_without_reordering_history() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('j'));

    assert_eq!(app.notepad_history.history_cursor, 1);
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["alpha".to_string(), "beta".to_string()]
    );
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "beta"
    ));
}

#[test]
fn handle_notepad_history_space_previews_selected_item() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.start_notepad_history();

    assert_eq!(app.notepad_history.history_cursor, 0);
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Idle
    ));

    app.handle_notepad_history(KeyCode::Char(' '));

    assert_eq!(app.notepad_history.history_cursor, 0);
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "alpha"
    ));
}

#[test]
fn handle_notepad_history_page_down_and_page_up_move_by_visible_page() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "zero".to_string(),
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
        "four".to_string(),
        "five".to_string(),
    ];
    app.notepad_history.page_size = 2;
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('j'));

    app.handle_notepad_history(KeyCode::PageDown);
    assert_eq!(app.notepad_history.history_cursor, 3);
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "three"
    ));

    app.handle_notepad_history(KeyCode::PageUp);
    assert_eq!(app.notepad_history.history_cursor, 1);
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "one"
    ));
}

#[test]
fn handle_notepad_history_starts_scrolling_before_cursor_reaches_view_edge() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "zero".to_string(),
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
        "four".to_string(),
        "five".to_string(),
        "six".to_string(),
        "seven".to_string(),
    ];
    app.notepad_history.page_size = 6;
    app.start_notepad_history();

    for _ in 0..4 {
        app.handle_notepad_history(KeyCode::Char('j'));
    }
    assert_eq!(app.notepad_history.history_cursor, 4);
    assert_eq!(app.notepad_history.history_state.offset(), 1);

    for _ in 0..2 {
        app.handle_notepad_history(KeyCode::Char('k'));
    }
    assert_eq!(app.notepad_history.history_cursor, 2);
    assert_eq!(app.notepad_history.history_state.offset(), 0);
}

#[test]
fn handle_notepad_history_page_up_at_top_does_not_repreview() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.notepad_history.page_size = 2;
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::PageUp);

    assert_eq!(app.notepad_history.history_cursor, 0);
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Idle
    ));
}
