use super::*;

#[test]
fn handle_patch_select_slash_then_chars_filter_and_preview_first_result() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "JK Brass/Bass 1.fxp"]);
    app.patch_filtered = vec![
        "Pads/Pad 1.fxp".to_string(),
        "JK Brass/Bass 1.fxp".to_string(),
    ];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(app.patch_query, "jk");
    assert_eq!(app.patch_cursor, 0);
    assert_eq!(app.patch_list_state.selected(), Some(0));
    assert_eq!(app.patch_filtered, vec!["JK Brass/Bass 1.fxp".to_string()]);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "JK Brass/Bass 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_enter_exits_filter_input_and_keeps_filtered_results() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "JK Brass/Bass 1.fxp", "JK Lead.fxp"]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert!(!app.patch_select_filter_active);
    assert_eq!(app.patch_query, "jk");
    assert_eq!(
        app.patch_filtered,
        vec!["JK Brass/Bass 1.fxp".to_string(), "JK Lead.fxp".to_string()]
    );
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "JK Lead.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_backspace_with_empty_query_exits_filter_input() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchSelect;
    app.patch_select_filter_active = true;

    app.handle_patch_select(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert!(!app.patch_select_filter_active);
}

#[test]
fn handle_patch_select_char_filters_and_previews_first_result_after_slash() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE));

    assert_eq!(app.patch_query, "L");
    assert_eq!(app.patch_filtered, vec!["Leads/Lead 1.fxp".to_string()]);
    assert_eq!(app.patch_cursor, 0);
    assert_eq!(app.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_backspace_to_empty_keeps_filter_input_active() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Leads/Lead 1.fxp".to_string()];
    app.patch_query = "L".to_string();
    app.patch_select_filter_active = true;
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(app.patch_query, "");
    assert_eq!(
        app.patch_filtered,
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()]
    );
    assert!(app.patch_select_filter_active);
    assert_eq!(app.patch_cursor, 0);
    assert_eq!(app.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#
    ));

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert!(app.patch_select_filter_active);
    assert_eq!(app.patch_query, "p");
    assert_eq!(
        app.patch_filtered,
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_question_mark_enters_help_and_esc_returns_to_patch_select() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::PatchSelect));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::PatchSelect));
}

#[test]
fn handle_patch_select_n_p_t_switch_to_corresponding_overlays() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Leads/Lead 1.fxp",
    ]))));
    app.patch_phrase_store.notepad.history = vec!["line history".to_string()];
    app.patch_phrase_store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["lead history".to_string()],
            favorites: vec!["lead favorite".to_string()],
        },
    );
    app.open_patch_select_overlay(None);
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert!(matches!(app.mode, Mode::NotepadHistory));

    app.open_patch_select_overlay(None);
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Leads/Lead 1.fxp"));

    app.open_patch_select_overlay(None);
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_filtered[app.patch_cursor], "Leads/Lead 1.fxp");
}

#[test]
fn open_patch_select_overlay_prefills_saved_patch_filter_from_current_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        r#"{"Surge XT patch":"Pads/Pad 2.fxp","Surge XT patch filter":"pads"} l8cdef"#.to_string(),
    ];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Pads/Pad 2.fxp",
        "Leads/Lead 1.fxp",
    ]))));

    app.open_patch_select_overlay(None);

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_query, "pads");
    assert_eq!(
        crate::text_input::textarea_value(&app.patch_query_textarea),
        "pads"
    );
    assert!(!app.patch_select_filter_active);
    assert_eq!(
        app.patch_filtered,
        vec!["Pads/Pad 1.fxp".to_string(), "Pads/Pad 2.fxp".to_string()]
    );
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
}
