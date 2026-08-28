use super::*;

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

#[test]
fn handle_normal_enter_uses_test_preview_path_when_plugin_entries_are_unavailable() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
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
            .map(|pos| pos.measure_index),
        Some(0)
    );
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: meas1")
    );
}
