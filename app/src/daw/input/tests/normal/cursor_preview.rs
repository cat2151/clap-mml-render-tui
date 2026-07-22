use super::*;

#[test]
fn handle_normal_h_and_j_preview_new_target_when_not_playing() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 2;
    app.editor.data[1][1] = "cdef".to_string();
    app.editor.data[2][1] = "gabc".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_measure, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback
            .position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index),
        Some(0)
    );

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_track, 2);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: meas1")
    );
}

#[test]
fn handle_normal_cursor_move_restarts_preview_on_new_target() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 2;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 1,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_measure, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.playback
            .position
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
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_measure, 2);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert!(app.playback.position.lock().unwrap().is_none());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_track, 1);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert!(app.playback.position.lock().unwrap().is_none());
}

#[test]
fn handle_normal_stops_preview_when_cursor_moves_to_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_measure, 0);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.playback.position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn handle_normal_stops_preview_when_cursor_moves_to_non_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.editor.cursor_track, 0);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.playback.position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}
