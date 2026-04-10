use super::*;

#[test]
fn handle_patch_select_f_adds_selected_patch_and_phrase_to_favorites() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));

    let stored = app
        .patch_phrase_store
        .patches
        .get("Leads/Lead 1.fxp")
        .expect("favorite should be stored for selected patch");
    assert_eq!(stored.favorites, vec!["l8cdef".to_string()]);
    assert_eq!(
        app.patch_favorite_items,
        vec!["Leads/Lead 1.fxp".to_string()]
    );
    assert!(app.patch_phrase_store_dirty);
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_query, "");
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_f_moves_newly_added_patch_to_favorites_top() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_phrase_store.favorite_patches = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));

    assert_eq!(
        app.patch_favorite_items,
        vec!["Leads/Lead 1.fxp".to_string(), "Pads/Pad 1.fxp".to_string()]
    );
}
