use super::*;

#[test]
fn handle_normal_dd_yanks_current_measure_clears_it_and_records_patch_history() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Pad 1.fxp"}"#.to_string();
    app.editor.data[1][1] = "cdef".to_string();
    app.playback.measure_mmls.lock().unwrap()[0] = "stale".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(app.editor.pending_delete);
    assert_eq!(app.editor.data[1][1], "cdef");
    assert!(app.editor.yank_buffer.is_none());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(!app.editor.pending_delete);
    assert_eq!(app.editor.data[1][1], "");
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("cdef"));
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["cdef".to_string()])
    );
    assert!(app.patch_phrase_store_dirty);
    assert_eq!(app.playback.measure_mmls.lock().unwrap()[0], "");
}

#[test]
fn handle_normal_p_overwrites_current_measure_from_yank_and_records_previous_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Pad 1.fxp"}"#.to_string();
    app.editor.data[1][1] = "old".to_string();
    app.editor.yank_buffer = Some("new".to_string());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.data[1][1], "new");
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("new"));
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["old".to_string()])
    );
    assert!(app.patch_phrase_store_dirty);
}

#[test]
fn handle_normal_p_logs_when_yank_buffer_is_empty() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][1] = "old".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.data[1][1], "old");
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("ヤンクバッファが空です")
    );
}

#[test]
fn handle_normal_u_restores_previous_init_measure_after_paste() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 0;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Init.fxp"}"#.to_string();
    app.editor.yank_buffer = Some(r#"{"Surge XT patch": "Pasted.fxp"}"#.to_string());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.data[1][0], r#"{"Surge XT patch": "Pasted.fxp"}"#);
    assert!(app.editor.paste_undo.is_some());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.data[1][0], r#"{"Surge XT patch": "Init.fxp"}"#);
    assert_eq!(
        app.editor.yank_buffer.as_deref(),
        Some(r#"{"Surge XT patch": "Pasted.fxp"}"#)
    );
    assert!(app.editor.paste_undo.is_none());
}
