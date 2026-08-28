use super::*;

#[test]
fn handle_history_overlay_arrow_and_space_preview_selected_mml() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["history".to_string()],
            favorites: vec!["favorite".to_string()],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Right);

    assert!(matches!(
        app.overlays.history.focus,
        DawHistoryPane::Favorites
    ));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}favorite"#
    );

    app.handle_history_overlay(KeyCode::Char(' '));

    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}favorite"#
    );

    app.handle_history_overlay(KeyCode::Left);

    assert!(matches!(
        app.overlays.history.focus,
        DawHistoryPane::History
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}history"#
    );
}

#[test]
fn handle_history_overlay_down_previews_next_history_item() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["first".to_string(), "second".to_string()],
            favorites: vec![],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Down);

    assert_eq!(app.overlays.history.history_cursor, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}second"#
    );
}

#[test]
fn handle_history_overlay_j_k_preview_uses_overlay_patch_name() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] =
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Bass/Bass 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["bass first".to_string(), "bass second".to_string()],
            favorites: vec![],
        },
    );
    app.start_history_overlay_for_patch_name(Some("Bass/Bass 1.fxp".to_string()));

    app.handle_history_overlay(KeyCode::Char('j'));

    assert_eq!(app.overlays.history.history_cursor, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Bass/Bass 1.fxp","Surge XT patch filter":"pads"}bass second"#
    );

    app.handle_history_overlay(KeyCode::Char('k'));

    assert_eq!(app.overlays.history.history_cursor, 0);
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Bass/Bass 1.fxp","Surge XT patch filter":"pads"}bass first"#
    );
}

#[test]
fn handle_history_overlay_j_k_preview_falls_back_when_track_init_json_is_not_object() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = "[]".to_string();
    app.patch_phrase_store.patches.insert(
        "Bass/Bass 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["bass first".to_string(), "bass second".to_string()],
            favorites: vec![],
        },
    );
    app.start_history_overlay_for_patch_name(Some("Bass/Bass 1.fxp".to_string()));

    app.handle_history_overlay(KeyCode::Char('j'));

    assert_eq!(app.overlays.history.history_cursor, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback.measure_track_mmls.lock().unwrap()[0][2],
        r#"{"Surge XT patch":"Bass/Bass 1.fxp"}bass second"#
    );
}
