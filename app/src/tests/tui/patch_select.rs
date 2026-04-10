use super::*;

#[test]
fn handle_patch_select_ctrl_j_moves_cursor_and_previews_destination_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));

    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_j_and_k_move_cursor_and_preview_destination_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp", "Bass/Bass 1.fxp"]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert_eq!(app.patch_query, "");
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

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
fn handle_patch_select_ctrl_p_moves_cursor_up() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));

    assert_eq!(app.patch_cursor, 0);
    assert_eq!(app.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn start_patch_select_builds_favorite_items_in_registered_order() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pad B", "Pad A",
    ]))));
    app.patch_phrase_store.favorite_patches = vec![
        "Pad 2".to_string(),
        "Pad A".to_string(),
        "Pad 11".to_string(),
    ];
    app.patch_phrase_store.patches.insert(
        "Pad A".to_string(),
        crate::history::PatchPhraseState {
            history: vec![],
            favorites: vec!["l8cdef".to_string()],
        },
    );
    app.patch_phrase_store.patches.insert(
        "Pad 11".to_string(),
        crate::history::PatchPhraseState {
            history: vec![],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_store.patches.insert(
        "Pad 2".to_string(),
        crate::history::PatchPhraseState {
            history: vec![],
            favorites: vec!["o4c".to_string()],
        },
    );

    app.start_patch_select();

    assert_eq!(
        app.patch_favorite_items,
        vec![
            "Pad 2".to_string(),
            "Pad A".to_string(),
            "Pad 11".to_string()
        ]
    );
}

#[test]
fn start_patch_select_migrates_prefixed_favorites_from_legacy_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["hist".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );

    app.start_patch_select();

    assert_eq!(
        app.patch_favorite_items,
        vec!["patches_factory/Pads/Pad 1.fxp".to_string()]
    );
    assert!(app
        .patch_phrase_store
        .patches
        .contains_key("patches_factory/Pads/Pad 1.fxp"));
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
}

#[test]
fn open_patch_select_overlay_selects_requested_initial_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Leads/Lead 1.fxp",
        "Bass/Bass 1.fxp",
    ]))));

    app.open_patch_select_overlay(Some("Leads/Lead 1.fxp"));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_filtered,
        vec![
            "Pads/Pad 1.fxp".to_string(),
            "Leads/Lead 1.fxp".to_string(),
            "Bass/Bass 1.fxp".to_string()
        ]
    );
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
    assert_eq!(app.patch_select_focus, PatchSelectPane::Patches);
}

#[test]
fn handle_patch_select_ctrl_n_and_ctrl_k_move_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pad 1"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pad 0", "Pad 1", "Pad 2"]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert_eq!(app.patch_cursor, 2);
    assert_eq!(app.patch_list_state.selected(), Some(2));

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
}

#[test]
fn handle_patch_select_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pad 0"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&[
        "Pad 0", "Pad 1", "Pad 2", "Pad 3", "Pad 4", "Pad 5", "Pad 6",
    ]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_select_page_size = 3;
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(app.patch_cursor, 4);
    assert_eq!(app.patch_list_state.selected(), Some(4));

    app.handle_patch_select(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
}

#[test]
fn handle_patch_select_starts_scrolling_before_cursor_reaches_view_edge() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pad 0"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&[
        "Pad 0", "Pad 1", "Pad 2", "Pad 3", "Pad 4", "Pad 5", "Pad 6", "Pad 7",
    ]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_select_page_size = 6;
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    for _ in 0..4 {
        app.handle_patch_select(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    }
    assert_eq!(app.patch_cursor, 4);
    assert_eq!(app.patch_list_state.offset(), 1);

    for _ in 0..2 {
        app.handle_patch_select(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    }
    assert_eq!(app.patch_cursor, 2);
    assert_eq!(app.patch_list_state.offset(), 0);
}

#[test]
fn handle_patch_select_l_moves_focus_to_favorites_and_previews_selected_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_phrase_store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![],
            favorites: vec!["l8cdef".to_string()],
        },
    );
    app.patch_favorite_items = vec!["Leads/Lead 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.patch_favorites_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));

    assert_eq!(app.patch_select_focus, PatchSelectPane::Favorites);
    assert_eq!(app.patch_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_select_page_down_moves_favorites_when_favorites_pane_is_focused() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Fav 0"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Fav 0", "Fav 1", "Fav 2", "Fav 3"]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    for patch in ["Fav 0", "Fav 1", "Fav 2", "Fav 3"] {
        app.patch_phrase_store.patches.insert(
            patch.to_string(),
            crate::history::PatchPhraseState {
                history: vec![],
                favorites: vec!["l8cdef".to_string()],
            },
        );
    }
    app.patch_favorite_items = vec![
        "Fav 0".to_string(),
        "Fav 1".to_string(),
        "Fav 2".to_string(),
        "Fav 3".to_string(),
    ];
    app.patch_select_focus = PatchSelectPane::Favorites;
    app.patch_select_page_size = 2;
    app.patch_favorites_cursor = 0;
    app.patch_favorites_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));

    assert_eq!(app.patch_favorites_cursor, 2);
    assert_eq!(app.patch_favorites_state.selected(), Some(2));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Fav 2"} l8cdef"#
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
fn handle_patch_select_backspace_refilters_and_previews_first_result() {
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
    assert!(!app.patch_select_filter_active);
    assert_eq!(app.patch_cursor, 0);
    assert_eq!(app.patch_list_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#
    ));
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
