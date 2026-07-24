use super::*;

#[test]
fn handle_normal_enter_records_notepad_history() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["l8cdef".to_string()];

    app.handle_normal(KeyCode::Enter);

    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["l8cdef".to_string()]
    );
}

#[test]
fn handle_patch_select_enter_records_notepad_history() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["cde".to_string()];
    app.patch_select.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.editor.lines,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
}

#[test]
fn handle_patch_phrase_enter_records_notepad_history() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
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
