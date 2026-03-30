use super::*;

#[test]
fn handle_normal_shift_h_opens_patch_history_overlay_for_track_patch() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let result = app.handle_normal(KeyCode::Char('H'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(app.cursor_track, 1);
    assert_eq!(
        app.history_overlay_patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert!(matches!(app.history_overlay_focus, DawHistoryPane::History));
    assert_eq!(app.history_overlay_history_cursor, 0);
    assert_eq!(app.history_overlay_favorites_cursor, 0);
}

#[test]
fn handle_normal_h_moves_measure_cursor_left() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_measure = 2;
    let cursor_track = app.cursor_track;

    app.handle_normal(KeyCode::Char('h'));

    assert_eq!(app.cursor_measure, 1);
    assert_eq!(app.cursor_track, cursor_track);
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_history_overlay_enter_overwrites_measure_and_backs_up_old_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.data[1][2] = "before".to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["after".to_string()],
            favorites: vec![],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Enter);

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.data[1][2], "after");
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pads/Pad 1.fxp")
            .expect("patch history")
            .history,
        vec!["before".to_string(), "after".to_string()]
    );
    assert!(app.patch_phrase_store_dirty);
}

#[test]
fn handle_normal_shift_h_without_track_patch_opens_filtered_history_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.patch_phrase_store.notepad.history = vec![
        "plain phrase".to_string(),
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string(),
    ];

    app.handle_normal(KeyCode::Char('H'));

    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(app.history_overlay_patch_name, None);
    assert_eq!(
        app.history_overlay_history_items(),
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
}

#[test]
fn handle_history_overlay_enter_without_track_patch_sets_patch_and_backs_up_old_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    app.data[1][2] = "before".to_string();
    app.patch_phrase_store.notepad.history =
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Enter);

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.data[1][0], r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#);
    assert_eq!(app.data[1][2], "l8cdef");
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} before"#.to_string(),
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()
        ]
    );
    assert!(app.patch_phrase_store_dirty);
}

#[test]
fn handle_history_overlay_enter_from_favorites_uses_selected_favorite() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["history".to_string()],
            favorites: vec!["favorite".to_string()],
        },
    );
    app.start_history_overlay();
    app.handle_history_overlay(KeyCode::Char('l'));

    app.handle_history_overlay(KeyCode::Enter);

    assert_eq!(app.data[1][1], "favorite");
}

#[test]
fn handle_history_overlay_arrow_and_space_preview_selected_mml() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["history".to_string()],
            favorites: vec!["favorite".to_string()],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Right);

    assert!(matches!(
        app.history_overlay_focus,
        DawHistoryPane::Favorites
    ));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch": "Pads/Pad 1.fxp"}favorite"#
    );

    app.handle_history_overlay(KeyCode::Char(' '));

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch": "Pads/Pad 1.fxp"}favorite"#
    );

    app.handle_history_overlay(KeyCode::Left);

    assert!(matches!(app.history_overlay_focus, DawHistoryPane::History));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch": "Pads/Pad 1.fxp"}history"#
    );
}

#[test]
fn handle_history_overlay_down_previews_next_history_item() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["first".to_string(), "second".to_string()],
            favorites: vec![],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Down);

    assert_eq!(app.history_overlay_history_cursor, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch": "Pads/Pad 1.fxp"}second"#
    );
}

#[test]
fn handle_history_overlay_slash_then_enter_keeps_filtered_results_for_j_navigation() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "alpha".to_string(),
                "beta jk".to_string(),
                "gamma jk".to_string(),
            ],
            favorites: vec![],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('/'));
    app.handle_history_overlay(KeyCode::Char('j'));
    app.handle_history_overlay(KeyCode::Char('k'));
    app.handle_history_overlay(KeyCode::Enter);
    app.handle_history_overlay(KeyCode::Char('j'));

    assert!(!app.history_overlay_filter_active);
    assert_eq!(app.history_overlay_query, "jk");
    assert_eq!(
        app.history_overlay_history_items(),
        vec!["beta jk".to_string(), "gamma jk".to_string()]
    );
    assert_eq!(app.history_overlay_history_cursor, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_measure_track_mmls.lock().unwrap()[0][1],
        r#"{"Surge XT patch": "Pads/Pad 1.fxp"}gamma jk"#
    );
}

#[test]
fn handle_history_overlay_allows_slash_character_in_filter_query() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "alpha".to_string(),
                "dir/name".to_string(),
                "dir other".to_string(),
            ],
            favorites: vec![],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('/'));
    app.handle_history_overlay(KeyCode::Char('/'));
    app.handle_history_overlay(KeyCode::Char('n'));

    assert!(app.history_overlay_filter_active);
    assert_eq!(app.history_overlay_query, "/n");
    assert_eq!(
        app.history_overlay_history_items(),
        vec!["dir/name".to_string()]
    );
}

#[test]
fn handle_history_overlay_question_mark_opens_help_and_esc_returns_to_history_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["history".to_string()],
            favorites: vec!["favorite".to_string()],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('?'));

    assert!(matches!(app.mode, DawMode::Help));
    assert!(matches!(app.help_origin, DawMode::History));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, DawMode::History));
}

#[test]
fn handle_history_overlay_n_p_t_switch_to_corresponding_overlays() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_history_overlay_n_p_t");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pads")).unwrap();
    std::fs::create_dir_all(tmp.join("Bass")).unwrap();
    std::fs::write(tmp.join("Pads").join("Pad 1.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Bass").join("Bass 1.fxp"), b"dummy").unwrap();

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dir: Some(tmp.to_string_lossy().into_owned()),
        ..(*app.cfg).clone()
    });
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.patch_phrase_store.notepad.history = vec![
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} selected phrase"#.to_string(),
        r#"{"Surge XT patch":"Bass/Bass 1.fxp"} bass phrase"#.to_string(),
    ];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["selected phrase".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );
    app.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('n'));
    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(app.history_overlay_patch_name, None);
    assert_eq!(app.history_overlay_history_cursor, 0);

    app.handle_history_overlay(KeyCode::Char('p'));
    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(
        app.history_overlay_patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );

    app.handle_history_overlay(KeyCode::Char('t'));
    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert_eq!(app.patch_filtered[app.patch_cursor], "Pads/Pad 1.fxp");
}

#[test]
fn handle_patch_select_enter_overwrites_current_track_init_patch() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_patch_select_enter");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pads")).unwrap();
    std::fs::create_dir_all(tmp.join("Bass")).unwrap();
    std::fs::write(tmp.join("Pads").join("Pad 1.fxp"), b"dummy").unwrap();
    std::fs::write(tmp.join("Bass").join("Bass 1.fxp"), b"dummy").unwrap();

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dir: Some(tmp.to_string_lossy().into_owned()),
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
