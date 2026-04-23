use super::*;

#[test]
fn handle_normal_shift_h_enters_patch_phrase_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
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
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Pads/Pad 1.fxp"));
    assert!(matches!(app.patch_phrase_focus, PatchPhrasePane::History));
    assert_eq!(app.patch_phrase_history_state.selected(), Some(0));
    assert_eq!(app.patch_phrase_favorites_state.selected(), Some(0));
}

#[test]
fn handle_normal_shift_h_without_patch_name_shows_notepad_history_guide() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["plain phrase".to_string()];
    app.patch_phrase_store.notepad.history = vec!["history phrase".to_string()];

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::NotepadHistoryGuide));
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));
}

#[test]
fn handle_notepad_history_guide_enter_opens_notepad_history_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["history phrase".to_string()];
    app.mode = Mode::NotepadHistoryGuide;

    app.handle_notepad_history_guide(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::NotepadHistory));
    assert_eq!(app.notepad_history_cursor, 0);
    assert_eq!(app.notepad_history_state.selected(), Some(0));
}

#[test]
fn handle_normal_h_no_longer_enters_notepad_history_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["l8cdef".to_string()];

    let result = app.handle_normal(KeyCode::Char('h'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_enter_records_notepad_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["l8cdef".to_string()];

    app.handle_normal(KeyCode::Enter);

    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["l8cdef".to_string()]
    );
}

#[test]
fn handle_patch_select_enter_records_notepad_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
}

#[test]
fn handle_patch_phrase_enter_records_notepad_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Enter);

    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
}

#[test]
fn handle_notepad_history_enter_overwrites_current_line_and_closes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad.history = vec!["after".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(app.lines, vec!["after".to_string()]);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "after"
    ));
}
