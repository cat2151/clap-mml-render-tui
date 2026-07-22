use super::*;

#[test]
fn handle_patch_select_enter_keeps_saved_patch_filter_on_selected_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"} l8cdef"#.to_string(),
    ];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Pads/Pad 2.fxp"]);
    app.patch_select.patch_filtered =
        vec!["Pads/Pad 1.fxp".to_string(), "Pads/Pad 2.fxp".to_string()];
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.editor.lines,
        vec![
            r#"{"Surge XT patch": "Pads/Pad 2.fxp", "Surge XT patch filter": "pads"} l8cdef"#
                .to_string()
        ]
    );
}

#[test]
fn handle_patch_select_enter_primes_returned_normal_line_into_cache() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all =
        make_patches(&["Pads/Pad 1.fxp", "Pads/Pad 2.fxp", "Pads/Pad 3.fxp"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_select_page_size = 2;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(app.mode, Mode::Normal));
    let cache = app.audio.cache.lock().unwrap();
    assert!(cache.contains_key(r#"{"Surge XT patch": "Pads/Pad 2.fxp"} l8cdef"#));
}
