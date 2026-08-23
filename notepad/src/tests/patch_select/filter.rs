use super::*;

#[test]
fn parent_dir_supplies_the_default_generic_display_path_query() {
    assert_eq!(
        NotepadScreen::default_display_path_filter_query_from_parent_dir(
            "patches_factory/Instrument/Soft Strum.fxp"
        )
        .as_deref(),
        Some("instrument")
    );
    assert_eq!(
        NotepadScreen::default_display_path_filter_query_from_parent_dir(
            "patches_3rdparty/Acme/Guitars/Clean Voice.fxp"
        )
        .as_deref(),
        Some("guitars")
    );
    assert_eq!(
        NotepadScreen::default_display_path_filter_query_from_parent_dir("No Category.fxp"),
        None
    );
}

#[test]
fn handle_patch_select_slash_then_chars_filter_and_preview_first_result() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "JK Brass/Bass 1.fxp"]);
    app.patch_select.patch_filtered = vec![
        "Pads/Pad 1.fxp".to_string(),
        "JK Brass/Bass 1.fxp".to_string(),
    ];
    app.patch_select.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(app.patch_select.patch_query, "jk");
    assert_eq!(app.patch_select.patch_cursor, 0);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(0));
    assert_eq!(
        app.patch_select.patch_filtered,
        vec!["JK Brass/Bass 1.fxp".to_string()]
    );
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "JK Brass/Bass 1.fxp", "Surge XT patch filter": "jk"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_enter_exits_filter_input_and_keeps_filtered_results() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all =
        make_patches(&["Pads/Pad 1.fxp", "JK Brass/Bass 1.fxp", "JK Lead.fxp"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert!(!app.patch_select.patch_select_filter_active);
    assert_eq!(app.patch_select.patch_query, "jk");
    assert_eq!(
        app.patch_select.patch_filtered,
        vec!["JK Brass/Bass 1.fxp".to_string(), "JK Lead.fxp".to_string()]
    );
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "JK Lead.fxp", "Surge XT patch filter": "jk"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_backspace_with_empty_query_keeps_filter_input_active() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::PatchSelect;
    app.patch_select.patch_select_filter_active = true;

    app.handle_patch_select(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert!(app.patch_select.patch_select_filter_active);
    assert_eq!(app.patch_select.patch_query, "");
}

#[test]
fn handle_patch_select_char_filters_and_previews_first_result_after_slash() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered =
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_select.patch_cursor = 1;
    app.patch_select.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE));

    assert_eq!(app.patch_select.patch_query, "L");
    assert_eq!(
        app.patch_select.patch_filtered,
        vec!["Leads/Lead 1.fxp".to_string()]
    );
    assert_eq!(app.patch_select.patch_cursor, 0);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp", "Surge XT patch filter": "L"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_backspace_to_empty_keeps_filter_input_active() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered = vec!["Leads/Lead 1.fxp".to_string()];
    app.patch_select.patch_query = "L".to_string();
    app.patch_select.patch_select_filter_active = true;
    app.patch_select.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(app.patch_select.patch_query, "");
    assert_eq!(
        app.patch_select.patch_filtered,
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()]
    );
    assert!(app.patch_select.patch_select_filter_active);
    assert_eq!(app.patch_select.patch_cursor, 0);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#
    ));

    app.handle_patch_select(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert!(app.patch_select.patch_select_filter_active);
    assert_eq!(app.patch_select.patch_query, "");

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert!(app.patch_select.patch_select_filter_active);
    assert_eq!(app.patch_select.patch_query, "p");
    assert_eq!(
        app.patch_select.patch_filtered,
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));

    assert!(app.patch_select.patch_select_filter_active);
    assert_eq!(app.patch_select.patch_query, "Xpad");
}

#[test]
fn open_patch_select_overlay_prefills_saved_patch_filter_from_current_line() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![
        r#"{"Surge XT patch":"Pads/Pad 2.fxp","Surge XT patch filter":"pads"} l8cdef"#.to_string(),
    ];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Pads/Pad 2.fxp",
        "Leads/Lead 1.fxp",
    ]))));

    app.open_patch_select_overlay(None);

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_select.patch_query, "pads");
    assert_eq!(
        cmrt_tui_core::text_input::textarea_value(&app.patch_select.patch_query_textarea),
        "pads"
    );
    assert!(!app.patch_select.patch_select_filter_active);
    assert_eq!(
        app.patch_select.patch_filtered,
        vec!["Pads/Pad 1.fxp".to_string(), "Pads/Pad 2.fxp".to_string()]
    );
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
}

#[test]
fn handle_patch_select_slash_on_favorites_filters_favorites_query_independently() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_select.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_select.patch_filtered = app
        .patch_select
        .patch_all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.patch_select.patch_favorite_items =
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_select.patch_select_focus = PatchSelectPane::Favorites;
    app.patch_select.patch_favorites_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    assert!(app.patch_select.patch_select_filter_active);
    assert_eq!(
        app.patch_select.patch_select_focus,
        PatchSelectPane::Favorites
    );
    assert_eq!(app.patch_select.patch_query, "");
    assert_eq!(app.patch_select.patch_favorites_query, "le");
    assert_eq!(
        app.patch_select_favorite_items(),
        vec!["Leads/Lead 1.fxp".to_string()]
    );
    assert_eq!(app.patch_select.patch_favorites_cursor, 0);
    assert_eq!(app.patch_select.patch_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}
