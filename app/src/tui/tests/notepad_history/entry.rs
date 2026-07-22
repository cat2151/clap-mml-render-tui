use super::*;

#[test]
fn handle_normal_shift_h_enters_patch_phrase_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(
        app.patch_phrase.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert!(matches!(app.patch_phrase.focus, PatchPhrasePane::History));
    assert_eq!(app.patch_phrase.history_state.selected(), Some(0));
    assert_eq!(app.patch_phrase.favorites_state.selected(), Some(0));
}

#[test]
fn handle_normal_shift_h_without_patch_name_shows_notepad_history_guide() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["plain phrase".to_string()];
    app.patch_phrase_store.notepad.history = vec!["history phrase".to_string()];

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::NotepadHistoryGuide));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Idle
    ));
}

#[test]
fn handle_notepad_history_guide_enter_opens_notepad_history_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["history phrase".to_string()];
    app.mode = Mode::NotepadHistoryGuide;

    app.handle_notepad_history_guide(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::NotepadHistory));
    assert_eq!(app.notepad_history.history_cursor, 0);
    assert_eq!(app.notepad_history.history_state.selected(), Some(0));
}

#[test]
fn handle_normal_h_no_longer_enters_notepad_history_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["l8cdef".to_string()];

    let result = app.handle_normal(KeyCode::Char('h'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Normal));
}
