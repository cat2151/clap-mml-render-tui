use super::*;

#[test]
fn handle_history_overlay_question_mark_opens_help_and_esc_returns_to_history_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
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
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
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
    app.editor.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Char('n'));
    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(app.overlays.history.patch_name, None);
    assert_eq!(app.overlays.history.history_cursor, 0);

    app.handle_history_overlay(KeyCode::Char('p'));
    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(
        app.overlays.history.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );

    app.handle_history_overlay(KeyCode::Char('t'));
    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert_eq!(
        app.overlays.patch_select.filtered[app.overlays.patch_select.cursor],
        "Pads/Pad 1.fxp"
    );
}
