use super::*;

#[test]
fn handle_normal_shift_h_opens_patch_history_overlay_for_track_patch() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 2;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let result = app.handle_normal(KeyCode::Char('H'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(app.editor.cursor_track, 2);
    assert_eq!(
        app.overlays.history.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert!(matches!(
        app.overlays.history.focus,
        DawHistoryPane::History
    ));
    assert_eq!(app.overlays.history.history_cursor, 0);
    assert_eq!(app.overlays.history.favorites_cursor, 0);
}

#[test]
fn handle_normal_shift_h_migrates_legacy_patch_name_to_factory_prefixed_patch_name() {
    let tmp = TempDirGuard::new("cmrt_test_history_overlay_patch_prefix");
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
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 2;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    app.handle_normal(KeyCode::Char('H'));

    assert_eq!(
        app.overlays.history.patch_name.as_deref(),
        Some("patches_factory/Pads/Pad 1.fxp")
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
fn handle_normal_h_moves_measure_cursor_left() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 2;
    let cursor_track = app.editor.cursor_track;

    app.handle_normal(KeyCode::Char('h'));

    assert_eq!(app.editor.cursor_measure, 1);
    assert_eq!(app.editor.cursor_track, cursor_track);
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_normal_shift_h_without_track_patch_opens_filtered_history_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.patch_phrase_store.notepad.history = vec![
        "plain phrase".to_string(),
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string(),
    ];

    app.handle_normal(KeyCode::Char('H'));

    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(app.overlays.history.patch_name, None);
    assert_eq!(
        app.history_overlay_history_items(),
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
}
