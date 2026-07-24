use super::*;

#[test]
fn handle_normal_shift_space_stops_current_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
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
fn handle_normal_shift_space_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.playback.position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn handle_normal_shift_enter_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.playback.position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn handle_normal_enter_stops_current_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
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
fn handle_normal_enter_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.playback.position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}
