use super::*;

#[test]
fn handle_patch_select_ctrl_j_moves_cursor_and_previews_destination_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered =
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_select.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));

    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_j_and_k_move_cursor_and_preview_destination_patch() {
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
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(app.patch_select.patch_query, "");
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_ctrl_p_moves_cursor_up() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered =
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));

    assert_eq!(app.patch_select.patch_cursor, 0);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_ctrl_n_and_ctrl_k_move_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pad 1"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pad 0", "Pad 1", "Pad 2"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert_eq!(app.patch_select.patch_cursor, 2);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(2));

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
}

#[test]
fn handle_patch_select_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pad 0"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&[
        "Pad 0", "Pad 1", "Pad 2", "Pad 3", "Pad 4", "Pad 5", "Pad 6",
    ]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_select_page_size = 3;
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(app.patch_select.patch_cursor, 4);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(4));

    app.handle_patch_select(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
}

#[test]
fn handle_patch_select_starts_scrolling_before_cursor_reaches_view_edge() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pad 0"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&[
        "Pad 0", "Pad 1", "Pad 2", "Pad 3", "Pad 4", "Pad 5", "Pad 6", "Pad 7",
    ]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_select_page_size = 6;
    app.patch_select.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    for _ in 0..4 {
        app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    }
    assert_eq!(app.patch_select.patch_cursor, 4);
    assert_eq!(app.patch_select.patch_list_state.offset(), 1);

    for _ in 0..2 {
        app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    }
    assert_eq!(app.patch_select.patch_cursor, 2);
    assert_eq!(app.patch_select.patch_list_state.offset(), 0);
}
