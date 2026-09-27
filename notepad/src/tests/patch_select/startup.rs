use super::*;

#[test]
fn open_patch_select_overlay_selects_requested_initial_patch_and_previews_it() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#,
        &["Pads/Pad 1.fxp", "Leads/Lead 1.fxp", "Bass/Bass 1.fxp"],
    );

    app.open_patch_select_overlay(Some("Leads/Lead 1.fxp"));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(selected(&app).as_deref(), Some("Leads/Lead 1.fxp"));
    assert_eq!(preset_label(&app), "lead");
    assert_eq!(
        playing(&app).as_deref(),
        Some(r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#)
    );
}

#[test]
fn opening_starts_on_the_current_line_patch_in_the_preset_that_matches_it() {
    let mut app = NotepadScreen::new_for_test(test_config());

    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Pads/Pad 2.fxp"} l8cdef"#,
        &["Pads/Pad 1.fxp", "Pads/Pad 2.fxp", "Leads/Lead 1.fxp"],
    );

    assert_eq!(selected(&app).as_deref(), Some("Pads/Pad 2.fxp"));
    assert_eq!(preset_label(&app), "pad");
    assert_eq!(
        patch_select_list(&app),
        vec!["Pads/Pad 1.fxp", "Pads/Pad 2.fxp"]
    );
}

#[test]
fn favorite_preset_lists_favorites_in_registered_order_and_is_not_chosen_on_open() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.favorite_patches = vec!["Tone 02".to_string(), "Tone 00".to_string()];
    add_favorite(&mut app, "Tone 02", "o4c");
    add_favorite(&mut app, "Tone 00", "l8cdef");
    // favorite_patches に載っていない favorite は、一覧順で後ろへ足される。
    add_favorite(&mut app, "Tone 01", "o5g");

    open_tones(&mut app, 3, "Tone 01");

    assert_eq!(preset_label(&app), "ALL");
    assert_eq!(selected(&app).as_deref(), Some("Tone 01"));
    choose_favorite_preset(&mut app);
    assert_eq!(
        patch_select_list(&app),
        vec!["Tone 02", "Tone 00", "Tone 01"]
    );
}

#[test]
fn start_patch_select_migrates_prefixed_favorites_from_legacy_patch_name() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["hist".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );

    open_patch_select_for_test(&mut app, "c", &["patches_factory/Pads/Pad 1.fxp"]);

    let select = app.patch_select.as_ref().unwrap();
    assert!(select.is_favorite("patches_factory/Pads/Pad 1.fxp"));
    assert!(app
        .patch_phrase_store
        .patches
        .contains_key("patches_factory/Pads/Pad 1.fxp"));
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
}
