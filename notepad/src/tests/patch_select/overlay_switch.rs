use super::*;

#[test]
fn handle_patch_select_question_mark_enters_help_and_esc_returns_to_patch_select() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::PatchSelect));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::PatchSelect));
}

#[test]
fn handle_patch_select_n_p_t_switch_to_corresponding_overlays() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Leads/Lead 1.fxp",
    ]))));
    app.patch_phrase_store.notepad.history = vec!["line history".to_string()];
    app.patch_phrase_store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["lead history".to_string()],
            favorites: vec!["lead favorite".to_string()],
        },
    );
    app.open_patch_select_overlay(None);
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert!(matches!(app.mode, Mode::NotepadHistory));

    app.open_patch_select_overlay(None);
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(
        app.patch_phrase.patch_name.as_deref(),
        Some("Leads/Lead 1.fxp")
    );

    app.open_patch_select_overlay(None);
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_select.patch_filtered[app.patch_select.patch_cursor],
        "Leads/Lead 1.fxp"
    );
}
