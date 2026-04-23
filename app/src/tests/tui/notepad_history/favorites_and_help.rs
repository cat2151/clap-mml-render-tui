use super::*;

#[test]
fn handle_notepad_history_f_adds_selected_history_to_favorites() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('j'));

    app.handle_notepad_history(KeyCode::Char('f'));

    assert_eq!(
        app.patch_phrase_store.notepad.favorites,
        vec!["beta".to_string()]
    );
}

#[test]
fn handle_notepad_history_right_switches_focus_to_favorites() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["beta".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Right);

    assert!(matches!(app.notepad_focus, PatchPhrasePane::Favorites));
    assert_eq!(app.notepad_history_state.selected(), Some(0));
    assert_eq!(app.notepad_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "beta"
    ));
}

#[test]
fn handle_notepad_history_left_switches_focus_to_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["beta".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Right);

    app.handle_notepad_history(KeyCode::Left);

    assert!(matches!(app.notepad_focus, PatchPhrasePane::History));
    assert_eq!(app.notepad_history_state.selected(), Some(0));
    assert_eq!(app.notepad_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "alpha"
    ));
}

#[test]
fn handle_notepad_history_dd_removes_favorite_and_moves_it_to_history_top() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["beta".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('l'));

    app.handle_notepad_history(KeyCode::Char('d'));
    assert!(app.notepad_pending_delete);
    app.handle_notepad_history(KeyCode::Char('d'));

    assert!(!app.notepad_pending_delete);
    assert!(app.patch_phrase_store.notepad.favorites.is_empty());
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["beta".to_string(), "alpha".to_string()]
    );
}

#[test]
fn handle_notepad_history_d_does_not_arm_delete_when_favorites_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('l'));

    app.handle_notepad_history(KeyCode::Char('d'));

    assert!(!app.notepad_pending_delete);
    assert_eq!(app.notepad_favorites_state.selected(), None);
}

#[test]
fn handle_notepad_history_question_mark_enters_help_and_esc_returns_to_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::NotepadHistory));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::NotepadHistory));
}
