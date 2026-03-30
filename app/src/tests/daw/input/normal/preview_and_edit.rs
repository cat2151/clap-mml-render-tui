use super::*;

#[test]
fn handle_normal_cursor_move_restarts_preview_on_new_target() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 1,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index),
        Some(0)
    );
    let logs = app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        logs.ends_with(&["preview: stop".to_string(), "preview: meas1".to_string()]),
        "logs: {:?}",
        logs
    );
}

#[test]
fn handle_normal_l_and_k_do_not_start_preview_while_playing() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 2;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 2);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert!(app.play_position.lock().unwrap().is_none());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_track, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert!(app.play_position.lock().unwrap().is_none());
}

#[test]
fn handle_normal_stops_preview_when_cursor_moves_to_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 0);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn handle_normal_stops_preview_when_cursor_moves_to_non_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_track, 0);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn normal_playback_shortcuts_map_correctly() {
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(NormalPlaybackShortcut::PreviewCurrentTrack)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        Some(NormalPlaybackShortcut::PreviewCurrentTrack)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Some(NormalPlaybackShortcut::PreviewAllTracks)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT)),
        Some(NormalPlaybackShortcut::PlayFromCursor)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)),
        Some(NormalPlaybackShortcut::TogglePlay)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE)),
        Some(NormalPlaybackShortcut::TogglePlay)
    );
}

#[test]
fn handle_normal_dd_yanks_current_measure_clears_it_and_records_patch_history() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "cdef".to_string();
    app.play_measure_mmls.lock().unwrap()[0] = "stale".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(app.normal_pending_delete);
    assert_eq!(app.data[1][1], "cdef");
    assert!(app.yank_buffer.is_none());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(!app.normal_pending_delete);
    assert_eq!(app.data[1][1], "");
    assert_eq!(app.yank_buffer.as_deref(), Some("cdef"));
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["cdef".to_string()])
    );
    assert!(app.patch_phrase_store_dirty);
    assert_eq!(app.play_measure_mmls.lock().unwrap()[0], "");
}

#[test]
fn handle_normal_p_overwrites_current_measure_from_yank_and_records_previous_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "old".to_string();
    app.yank_buffer = Some("new".to_string());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.data[1][1], "new");
    assert_eq!(app.yank_buffer.as_deref(), Some("new"));
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
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][1] = "old".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.data[1][1], "old");
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("ヤンクバッファが空です")
    );
}

#[test]
fn handle_normal_enter_stops_current_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn handle_normal_enter_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn handle_normal_enter_uses_test_preview_path_when_entry_ptr_is_unavailable() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|pos| pos.measure_index),
        Some(0)
    );
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: meas1")
    );
}
