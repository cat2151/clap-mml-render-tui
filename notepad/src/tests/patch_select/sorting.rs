use super::*;

#[test]
fn handle_patch_select_ctrl_s_toggles_sort_order_and_keeps_selected_patch() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines =
        vec![r#"{"Surge XT patch":"patches_factory/pad/Super Pad.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = vec![
        (
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/lead/super lead.fxp".to_string(),
        ),
        (
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_factory/pad/super pad.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/great lead.fxp".to_string(),
        ),
        (
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/great pad.fxp".to_string(),
        ),
    ];
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

    assert_eq!(
        app.patch_select.patch_select_sort_order,
        cmrt_patches::PatchSortOrder::Category
    );
    assert_eq!(
        app.patch_select.patch_filtered,
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
    assert_eq!(app.patch_select.patch_cursor, 2);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(2));
    assert_eq!(
        app.patch_select
            .patch_filtered
            .get(app.patch_select.patch_cursor)
            .map(String::as_str),
        Some("patches_factory/pad/Super Pad.fxp")
    );

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

    assert_eq!(
        app.patch_select.patch_select_sort_order,
        cmrt_patches::PatchSortOrder::Path
    );
    assert_eq!(
        app.patch_select.patch_filtered,
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
    assert_eq!(
        app.patch_select
            .patch_filtered
            .get(app.patch_select.patch_cursor)
            .map(String::as_str),
        Some("patches_factory/pad/Super Pad.fxp")
    );
}
