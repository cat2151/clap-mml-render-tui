use super::*;

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
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}gamma jk"#
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
fn handle_history_overlay_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('/'));
    app.handle_history_overlay(KeyCode::Char('p'));
    app.handle_history_overlay(KeyCode::Char('a'));
    app.handle_history_overlay(KeyCode::Char('d'));
    app.handle_history_overlay_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_history_overlay(KeyCode::Char('X'));

    assert!(app.history_overlay_filter_active);
    assert_eq!(app.history_overlay_query, "Xpad");
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
    let tmp = TempDirGuard::new("cmrt_test_handle_history_overlay_n_p_t");
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
