use super::*;

#[test]
fn handle_patch_phrase_n_p_t_switch_to_corresponding_overlays() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Leads/Lead 1.fxp",
    ]))));
    app.patch_phrase_store.notepad.history = vec!["from history".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::NotepadHistory));

    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('p'));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(
        app.patch_phrase.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );

    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('t'));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_select.patch_filtered[app.patch_select.patch_cursor],
        "Pads/Pad 1.fxp"
    );
}

#[test]
fn handle_patch_phrase_question_mark_enters_help_and_esc_returns_to_patch_phrase() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::PatchPhrase));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::PatchPhrase));
}
