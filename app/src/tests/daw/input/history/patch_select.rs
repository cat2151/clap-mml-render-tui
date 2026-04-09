use super::*;

#[test]
fn handle_patch_select_enter_overwrites_current_track_init_patch() {
    let tmp = TempDirGuard::new("cmrt_test_handle_patch_select_enter");
    std::fs::create_dir_all(tmp.path().join("Pads")).unwrap();
    std::fs::create_dir_all(tmp.path().join("Bass")).unwrap();
    std::fs::write(tmp.path().join("Pads").join("Pad 1.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.path().join("Bass").join("Bass 1.fxp"), b"dummy").unwrap();

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![tmp.path().to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
    app.cursor_track = 1;
    app.cursor_measure = 2;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][2] = "l8cdef".to_string();

    app.start_patch_select_overlay(Some("Bass/Bass 1.fxp"));
    app.handle_patch_select(KeyCode::Enter);

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.data[1][0], r#"{"Surge XT patch":"Bass/Bass 1.fxp"}"#);
    assert_eq!(app.data[1][2], "l8cdef");
}

#[test]
fn start_patch_select_overlay_migrates_prefixed_favorites_from_legacy_patch_name() {
    let tmp = TempDirGuard::new("cmrt_test_daw_patch_select_patch_prefix");
    let factory_patch = tmp
        .path()
        .join("patches_factory")
        .join("Pads")
        .join("Pad 1.fxp");
    std::fs::create_dir_all(factory_patch.parent().unwrap()).unwrap();
    std::fs::write(&factory_patch, b"dummy").unwrap();

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![tmp.path().to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["hist".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );

    app.start_patch_select_overlay(None);

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
fn start_patch_select_overlay_keeps_favorites_in_registered_order() {
    let tmp = TempDirGuard::new("cmrt_test_daw_patch_select_favorite_order");
    std::fs::create_dir_all(tmp.path()).unwrap();
    for path in ["Pad B.fxp", "Pad A.fxp", "Pad 2.fxp", "Pad 11.fxp"] {
        std::fs::write(tmp.path().join(path), b"dummy").unwrap();
    }

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![tmp.path().to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.patch_phrase_store.favorite_patches = vec![
        "Pad 2.fxp".to_string(),
        "Pad A.fxp".to_string(),
        "Pad 11.fxp".to_string(),
    ];
    for patch_name in ["Pad A.fxp", "Pad 2.fxp", "Pad 11.fxp"] {
        app.patch_phrase_store.patches.insert(
            patch_name.to_string(),
            crate::history::PatchPhraseState {
                history: vec![],
                favorites: vec!["fav".to_string()],
            },
        );
    }

    app.start_patch_select_overlay(None);

    assert_eq!(
        app.patch_favorite_items,
        vec![
            "Pad 2.fxp".to_string(),
            "Pad A.fxp".to_string(),
            "Pad 11.fxp".to_string(),
        ]
    );
}

#[test]
fn handle_patch_select_enter_saves_filter_query_in_track_init_json() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
        ("Lead 1.fxp".to_string(), "lead 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    for key in ['b', 'a', 's', 's'] {
        app.handle_patch_select(KeyCode::Char(key));
    }
    app.handle_patch_select(KeyCode::Enter);
    app.handle_patch_select(KeyCode::Enter);

    let init_json: serde_json::Value = serde_json::from_str(&app.data[1][0]).unwrap();
    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(init_json["Surge XT patch"], "Bass Soft 1.fxp");
    assert_eq!(init_json["Surge XT patch filter"], "bass");
}

#[test]
fn handle_patch_select_filter_space_adds_and_term_instead_of_previewing() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
        ("Bass Hard 1.fxp".to_string(), "bass hard 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('b'));
    app.handle_patch_select(KeyCode::Char('a'));
    app.handle_patch_select(KeyCode::Char('s'));
    app.handle_patch_select(KeyCode::Char('s'));
    let preview_before_space = app.play_measure_track_mmls.lock().unwrap()[0][1].clone();

    app.handle_patch_select(KeyCode::Char(' '));
    assert_eq!(app.patch_query, "bass ");
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        preview_before_space
    );
    app.handle_patch_select(KeyCode::Char('s'));
    app.handle_patch_select(KeyCode::Char('o'));
    app.handle_patch_select(KeyCode::Char('f'));
    app.handle_patch_select(KeyCode::Char('t'));

    assert!(app.patch_select_filter_active);
    assert_eq!(app.patch_query, "bass soft");
    assert_eq!(app.patch_filtered, vec!["Bass Soft 1.fxp".to_string()]);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Bass Soft 1.fxp"}l8cdef"#,
    );
}

#[test]
fn handle_patch_select_j_and_k_move_selection_until_slash_starts_filter_input() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        (
            "JK Brass/Bass 1.fxp".to_string(),
            "jk brass/bass 1.fxp".to_string(),
        ),
        ("JK Lead.fxp".to_string(), "jk lead.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_cursor = 1;
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('j'));
    app.handle_patch_select(KeyCode::Char('k'));

    assert_eq!(app.patch_query, "");
    assert!(!app.patch_select_filter_active);
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"JK Brass/Bass 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('j'));
    app.handle_patch_select(KeyCode::Char('k'));

    assert!(app.patch_select_filter_active);
    assert_eq!(app.patch_query, "jk");
    assert_eq!(
        app.patch_filtered,
        vec!["JK Brass/Bass 1.fxp".to_string(), "JK Lead.fxp".to_string()]
    );
    assert_eq!(app.patch_cursor, 0);
}

#[test]
fn handle_patch_select_left_in_filter_query_does_not_repreview() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('b'));
    let preview_before = app.play_measure_track_mmls.lock().unwrap()[0][1].clone();

    app.handle_patch_select(KeyCode::Left);

    assert!(app.patch_select_filter_active);
    assert_eq!(app.patch_query, "b");
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        preview_before
    );
}

#[test]
fn handle_patch_select_arrow_keys_move_selection_in_left_pane() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
        ("Lead/Lead 1.fxp".to_string(), "lead/lead 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_cursor = 1;
    app.mode = DawMode::PatchSelect;
    app.patch_select_focus = DawPatchSelectPane::Patches;

    app.handle_patch_select(KeyCode::Down);
    assert_eq!(app.patch_cursor, 2);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Lead/Lead 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Up);
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Bass/Bass 1.fxp"}l8cdef"#,
    );
}

#[test]
fn handle_patch_select_j_k_preview_after_favorites_switch() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
        ("Lead/Lead 1.fxp".to_string(), "lead/lead 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.patch_favorite_items = vec!["Lead/Lead 1.fxp".to_string()];
    app.patch_select_focus = DawPatchSelectPane::Favorites;
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('h'));
    assert!(matches!(
        app.patch_select_focus,
        DawPatchSelectPane::Patches
    ));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Char('j'));
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Bass/Bass 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Char('k'));
    assert_eq!(app.patch_cursor, 0);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}l8cdef"#,
    );
}

#[test]
fn handle_patch_select_esc_cancels_filter_input_without_closing_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
        ("Bass Hard 1.fxp".to_string(), "bass hard 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    for key in ['b', 'a', 's', 's'] {
        app.handle_patch_select(KeyCode::Char(key));
    }
    app.handle_patch_select(KeyCode::Enter);
    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('x'));
    app.handle_patch_select(KeyCode::Esc);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(!app.patch_select_filter_active);
    assert_eq!(app.patch_query, "bass");
    assert_eq!(
        app.patch_filtered,
        vec!["Bass Soft 1.fxp".to_string(), "Bass Hard 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_backspace_with_empty_query_keeps_filter_input_active() {
    let (mut app, _cache_rx) = build_test_app();
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = app.patch_all.iter().map(|(name, _)| name.clone()).collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Backspace);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(app.patch_select_filter_active);
    assert_eq!(app.patch_query, "");
    assert_eq!(app.patch_filtered, vec!["Pads/Pad 1.fxp".to_string()]);
}

#[test]
fn handle_patch_select_allows_slash_character_in_filter_query_without_resetting_restore_point() {
    let (mut app, _cache_rx) = build_test_app();
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Soft 1.fxp".to_string(), "bass/soft 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
    ];
    app.patch_filtered = vec!["Bass/Soft 1.fxp".to_string(), "Bass Soft 1.fxp".to_string()];
    app.patch_query = "bass".to_string();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('s'));
    app.handle_patch_select(KeyCode::Esc);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(!app.patch_select_filter_active);
    assert_eq!(app.patch_query, "bass");
    assert_eq!(
        app.patch_filtered,
        vec!["Bass/Soft 1.fxp".to_string(), "Bass Soft 1.fxp".to_string()]
    );
}
