use super::*;

#[test]
fn handle_patch_select_j_prefetches_direction_first_then_fills_remaining_navigation_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&[
        "Pads/Pad 0.fxp",
        "Pads/Pad 1.fxp",
        "Pads/Pad 2.fxp",
        "Pads/Pad 3.fxp",
        "Pads/Pad 4.fxp",
        "Pads/Pad 5.fxp",
        "Pads/Pad 6.fxp",
        "Pads/Pad 7.fxp",
        "Pads/Pad 8.fxp",
        "Pads/Pad 9.fxp",
        "Pads/Pad 10.fxp",
        "Pads/Pad 11.fxp",
    ]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_cursor = 4;
    app.patch_select.patch_select_page_size = 5;
    app.patch_select.patch_list_state.select(Some(4));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert_eq!(
        app.audio
            .order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            r#"{"Surge XT patch": "Pads/Pad 6.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 7.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 4.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 10.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 0.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 8.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 9.fxp"} l8cdef"#,
        ]
    );
}

#[test]
fn handle_patch_select_k_prefetches_page_up_before_page_down_then_far_direction_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&[
        "Pads/Pad 0.fxp",
        "Pads/Pad 1.fxp",
        "Pads/Pad 2.fxp",
        "Pads/Pad 3.fxp",
        "Pads/Pad 4.fxp",
        "Pads/Pad 5.fxp",
        "Pads/Pad 6.fxp",
        "Pads/Pad 7.fxp",
        "Pads/Pad 8.fxp",
        "Pads/Pad 9.fxp",
        "Pads/Pad 10.fxp",
        "Pads/Pad 11.fxp",
    ]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_cursor = 6;
    app.patch_select.patch_select_page_size = 5;
    app.patch_select.patch_list_state.select(Some(6));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(
        app.audio
            .order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            r#"{"Surge XT patch": "Pads/Pad 4.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 3.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 6.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 0.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 10.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 2.fxp"} l8cdef"#,
            r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#,
        ]
    );
}
