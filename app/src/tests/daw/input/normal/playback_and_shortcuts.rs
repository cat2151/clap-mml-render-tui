use super::*;

#[test]
fn handle_normal_shift_space_stops_current_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

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
fn handle_normal_shift_space_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

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
fn handle_normal_shift_enter_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

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
fn preview_target_tracks_can_force_current_track_even_when_solo_mode_differs() {
    let target_tracks = preview_target_tracks(3, 2, false).expect("playable current track");

    assert_eq!(target_tracks, vec![2]);
}

#[test]
fn preview_target_tracks_can_temporarily_open_all_tracks() {
    let target_tracks = preview_target_tracks(3, 2, true).expect("all-track preview");

    assert_eq!(target_tracks, vec![1, 2]);
}

#[test]
fn preview_target_tracks_rejects_non_playable_current_track() {
    assert_eq!(preview_target_tracks(3, 0, false), None);
}

#[test]
fn play_from_cursor_uses_cursor_measure_index_for_start_position() {
    let start_measure_index =
        resolve_playback_start_measure_index(Some(1), NormalPlaybackShortcut::PlayFromCursor);

    assert_eq!(start_measure_index, Some(1));
}

#[test]
fn preview_shortcuts_keep_default_playback_start_position() {
    let start_measure_index =
        resolve_playback_start_measure_index(Some(1), NormalPlaybackShortcut::PreviewCurrentTrack);

    assert_eq!(start_measure_index, Some(0));
}
