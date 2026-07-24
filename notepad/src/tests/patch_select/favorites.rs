use super::*;

#[test]
fn handle_patch_select_l_moves_focus_to_favorites_and_previews_selected_patch() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered =
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_phrase_store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec![],
            favorites: vec!["l8cdef".to_string()],
        },
    );
    app.patch_select.patch_favorite_items = vec!["Leads/Lead 1.fxp".to_string()];
    app.patch_select.patch_list_state.select(Some(0));
    app.patch_select.patch_favorites_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));

    assert_eq!(
        app.patch_select.patch_select_focus,
        PatchSelectPane::Favorites
    );
    assert_eq!(app.patch_select.patch_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_page_down_moves_favorites_when_favorites_pane_is_focused() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Fav 0"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Fav 0", "Fav 1", "Fav 2", "Fav 3"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    for patch in ["Fav 0", "Fav 1", "Fav 2", "Fav 3"] {
        app.patch_phrase_store.patches.insert(
            patch.to_string(),
            cmrt_history::PatchPhraseState {
                history: vec![],
                favorites: vec!["l8cdef".to_string()],
            },
        );
    }
    app.patch_select.patch_favorite_items = vec![
        "Fav 0".to_string(),
        "Fav 1".to_string(),
        "Fav 2".to_string(),
        "Fav 3".to_string(),
    ];
    app.patch_select.patch_select_focus = PatchSelectPane::Favorites;
    app.patch_select.patch_select_page_size = 2;
    app.patch_select.patch_favorites_cursor = 0;
    app.patch_select.patch_favorites_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));

    assert_eq!(app.patch_select.patch_favorites_cursor, 2);
    assert_eq!(app.patch_select.patch_favorites_state.selected(), Some(2));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Fav 2"} l8cdef"#
    ));
}
