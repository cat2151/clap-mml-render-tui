use super::*;

#[test]
fn handle_history_overlay_slash_then_enter_keeps_filtered_results_for_j_navigation() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
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

    assert!(!app.overlays.history.filter_active);
    assert_eq!(app.overlays.history.query, "jk");
    assert_eq!(
        app.history_overlay_history_items(),
        vec!["beta jk".to_string(), "gamma jk".to_string()]
    );
    assert_eq!(app.overlays.history.history_cursor, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}gamma jk"#
    );
}

#[test]
fn handle_history_overlay_allows_slash_character_in_filter_query() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
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

    assert!(app.overlays.history.filter_active);
    assert_eq!(app.overlays.history.query, "/n");
    assert_eq!(
        app.history_overlay_history_items(),
        vec!["dir/name".to_string()]
    );
}

#[test]
fn handle_history_overlay_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('/'));
    app.handle_history_overlay(KeyCode::Char('p'));
    app.handle_history_overlay(KeyCode::Char('a'));
    app.handle_history_overlay(KeyCode::Char('d'));
    app.handle_history_overlay_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_history_overlay(KeyCode::Char('X'));

    assert!(app.overlays.history.filter_active);
    assert_eq!(app.overlays.history.query, "Xpad");
}
