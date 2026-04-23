use super::*;

#[test]
fn handle_patch_select_enter_keeps_saved_patch_filter_on_selected_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"} l8cdef"#.to_string(),
    ];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Pads/Pad 2.fxp"]);
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Pads/Pad 2.fxp".to_string()];
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.lines,
        vec![
            r#"{"Surge XT patch": "Pads/Pad 2.fxp", "Surge XT patch filter": "pads"} l8cdef"#
                .to_string()
        ]
    );
}

#[test]
fn handle_patch_select_enter_primes_returned_normal_line_into_cache() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.patch_all = make_patches(&["Pads/Pad 1.fxp", "Pads/Pad 2.fxp", "Pads/Pad 3.fxp"]);
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_cursor = 1;
    app.patch_select_page_size = 2;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(app.mode, Mode::Normal));
    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key(r#"{"Surge XT patch": "Pads/Pad 2.fxp"} l8cdef"#));
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
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} l8cdef"#
    ));
    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key(r#"{"Surge XT patch": "Pads/Pad 1.fxp"} l8cdef"#));
    assert!(cache.contains_key(r#"{"Surge XT patch": "Bass/Bass 1.fxp"} l8cdef"#));
}

#[test]
fn handle_patch_select_ctrl_s_toggles_sort_order_and_keeps_selected_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines =
        vec![r#"{"Surge XT patch":"patches_factory/pad/Super Pad.fxp"} l8cdef"#.to_string()];
    app.patch_all = vec![
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
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_cursor = 1;
    app.patch_list_state.select(Some(1));
    app.mode = Mode::PatchSelect;

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

    assert_eq!(
        app.patch_select_sort_order,
        crate::patches::PatchSortOrder::Category
    );
    assert_eq!(
        app.patch_filtered,
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
    assert_eq!(app.patch_cursor, 2);
    assert_eq!(app.patch_list_state.selected(), Some(2));
    assert_eq!(
        app.patch_filtered.get(app.patch_cursor).map(String::as_str),
        Some("patches_factory/pad/Super Pad.fxp")
    );

    app.handle_patch_select(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

    assert_eq!(
        app.patch_select_sort_order,
        crate::patches::PatchSortOrder::Path
    );
    assert_eq!(
        app.patch_filtered,
        vec![
            "patches_factory/lead/Super Lead.fxp".to_string(),
            "patches_factory/pad/Super Pad.fxp".to_string(),
            "patches_3rdparty/john/lead/Great Lead.fxp".to_string(),
            "patches_3rdparty/john/pad/Great Pad.fxp".to_string(),
        ]
    );
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
    assert_eq!(
        app.patch_filtered.get(app.patch_cursor).map(String::as_str),
        Some("patches_factory/pad/Super Pad.fxp")
    );
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
