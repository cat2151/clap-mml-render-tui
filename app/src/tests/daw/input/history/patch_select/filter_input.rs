use super::*;

#[test]
fn handle_patch_select_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.overlays.patch_select.all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
    ];
    app.overlays.patch_select.filtered = app
        .overlays
        .patch_select
        .all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('p'));
    app.handle_patch_select(KeyCode::Char('a'));
    app.handle_patch_select(KeyCode::Char('d'));
    app.handle_patch_select_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_patch_select(KeyCode::Char('X'));

    assert!(app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "Xpad");
}

#[test]
fn handle_patch_select_arrow_keys_move_selection_in_left_pane() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.overlays.patch_select.all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
        ("Lead/Lead 1.fxp".to_string(), "lead/lead 1.fxp".to_string()),
    ];
    app.overlays.patch_select.filtered = app
        .overlays
        .patch_select
        .all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.overlays.patch_select.cursor = 1;
    app.mode = DawMode::PatchSelect;
    app.overlays.patch_select.focus = DawPatchSelectPane::Patches;

    app.handle_patch_select(KeyCode::Down);
    assert_eq!(app.overlays.patch_select.cursor, 2);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Lead/Lead 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Up);
    assert_eq!(app.overlays.patch_select.cursor, 1);
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
    app.overlays.patch_select.all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
        ("Lead/Lead 1.fxp".to_string(), "lead/lead 1.fxp".to_string()),
    ];
    app.overlays.patch_select.filtered = app
        .overlays
        .patch_select
        .all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.overlays.patch_select.favorite_items = vec!["Lead/Lead 1.fxp".to_string()];
    app.overlays.patch_select.focus = DawPatchSelectPane::Favorites;
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('h'));
    assert!(matches!(
        app.overlays.patch_select.focus,
        DawPatchSelectPane::Patches
    ));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Char('j'));
    assert_eq!(app.overlays.patch_select.cursor, 1);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Bass/Bass 1.fxp"}l8cdef"#,
    );

    app.handle_patch_select(KeyCode::Char('k'));
    assert_eq!(app.overlays.patch_select.cursor, 0);
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
    app.overlays.patch_select.all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
        ("Bass Hard 1.fxp".to_string(), "bass hard 1.fxp".to_string()),
    ];
    app.overlays.patch_select.filtered = app
        .overlays
        .patch_select
        .all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
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
    assert!(!app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "bass");
    assert_eq!(
        app.overlays.patch_select.filtered,
        vec!["Bass Soft 1.fxp".to_string(), "Bass Hard 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_backspace_with_empty_query_keeps_filter_input_active() {
    let (mut app, _cache_rx) = build_test_app();
    app.overlays.patch_select.all =
        vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.overlays.patch_select.filtered = app
        .overlays
        .patch_select
        .all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Backspace);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "");
    assert_eq!(
        app.overlays.patch_select.filtered,
        vec!["Pads/Pad 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_backspace_to_empty_keeps_filter_input_active() {
    let (mut app, _cache_rx) = build_test_app();
    app.overlays.patch_select.all = vec![
        ("Keys/Keys 1.fxp".to_string(), "keys/keys 1.fxp".to_string()),
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
    ];
    app.overlays.patch_select.filtered = vec!["Keys/Keys 1.fxp".to_string()];
    app.overlays.patch_select.query = "k".to_string();
    app.overlays.patch_select.query_textarea = crate::text_input::new_single_line_textarea("k");
    app.overlays.patch_select.filter_active = true;
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Backspace);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "");
    assert_eq!(
        app.overlays.patch_select.filtered,
        vec!["Keys/Keys 1.fxp".to_string(), "Pads/Pad 1.fxp".to_string()]
    );

    app.handle_patch_select(KeyCode::Backspace);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "");

    app.handle_patch_select(KeyCode::Char('p'));

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "p");
    assert_eq!(
        app.overlays.patch_select.filtered,
        vec!["Keys/Keys 1.fxp".to_string(), "Pads/Pad 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_allows_slash_character_in_filter_query_without_resetting_restore_point() {
    let (mut app, _cache_rx) = build_test_app();
    app.overlays.patch_select.all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Soft 1.fxp".to_string(), "bass/soft 1.fxp".to_string()),
        ("Bass Soft 1.fxp".to_string(), "bass soft 1.fxp".to_string()),
    ];
    app.overlays.patch_select.filtered =
        vec!["Bass/Soft 1.fxp".to_string(), "Bass Soft 1.fxp".to_string()];
    app.overlays.patch_select.query = "bass".to_string();
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('s'));
    app.handle_patch_select(KeyCode::Esc);

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert!(!app.overlays.patch_select.filter_active);
    assert_eq!(app.overlays.patch_select.query, "bass");
    assert_eq!(
        app.overlays.patch_select.filtered,
        vec!["Bass/Soft 1.fxp".to_string(), "Bass Soft 1.fxp".to_string()]
    );
}

#[test]
fn handle_patch_select_slash_on_favorites_filters_favorites_query_independently() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "l8cdef".to_string();
    app.overlays.patch_select.all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        (
            "Leads/Lead 1.fxp".to_string(),
            "leads/lead 1.fxp".to_string(),
        ),
    ];
    app.overlays.patch_select.filtered = app
        .overlays
        .patch_select
        .all
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    app.overlays.patch_select.favorite_items =
        vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.overlays.patch_select.focus = DawPatchSelectPane::Favorites;
    app.mode = DawMode::PatchSelect;

    app.handle_patch_select(KeyCode::Char('/'));
    app.handle_patch_select(KeyCode::Char('l'));
    app.handle_patch_select(KeyCode::Char('e'));

    assert!(app.overlays.patch_select.filter_active);
    assert!(matches!(
        app.overlays.patch_select.focus,
        DawPatchSelectPane::Favorites
    ));
    assert_eq!(app.overlays.patch_select.query, "");
    assert_eq!(app.overlays.patch_select.favorites_query, "le");
    assert_eq!(
        app.patch_select_favorite_items(),
        vec!["Leads/Lead 1.fxp".to_string()]
    );
    assert_eq!(app.overlays.patch_select.favorites_cursor, 0);
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch":"Leads/Lead 1.fxp"}l8cdef"#,
    );
}
