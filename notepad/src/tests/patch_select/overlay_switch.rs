use super::*;

#[test]
fn question_mark_enters_help_and_esc_returns_to_patch_select() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 2, "Tone 00");

    press(&mut app, KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::PatchSelect));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert!(app.patch_select.is_some());
}

#[test]
fn n_p_t_switch_to_corresponding_overlays() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#,
        &["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"],
    );
    app.patch_phrase_store.notepad.history = vec!["line history".to_string()];
    app.patch_phrase_store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["lead history".to_string()],
            favorites: vec!["lead favorite".to_string()],
        },
    );

    app.open_patch_select_overlay(Some("Leads/Lead 1.fxp"));
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::NotepadHistory));
    assert!(app.patch_select.is_none());

    app.open_patch_select_overlay(Some("Leads/Lead 1.fxp"));
    press(&mut app, KeyCode::Char('p'));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(
        app.patch_phrase.patch_name.as_deref(),
        Some("Leads/Lead 1.fxp")
    );

    app.open_patch_select_overlay(Some("Leads/Lead 1.fxp"));
    press(&mut app, KeyCode::Char('t'));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(selected(&app).as_deref(), Some("Leads/Lead 1.fxp"));
}

#[test]
fn n_p_t_are_typed_into_the_regex_while_editing() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 2, "Tone 00");

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "npt?");

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(regex_text(&app), "npt?");
}
