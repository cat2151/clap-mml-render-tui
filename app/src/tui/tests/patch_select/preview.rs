use super::*;

#[test]
fn handle_patch_select_space_previews_current_selection_without_moving() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all =
        make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp", "Bass/Bass 1.fxp"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.patch_select.patch_select_page_size = 2;
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
    let cache = app.audio.cache.lock().unwrap();
    assert!(cache.contains_key(r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#));
    assert!(cache.contains_key(r#"{"Surge XT patch": "Bass/Bass 1.fxp"} l8cdef"#));
}
