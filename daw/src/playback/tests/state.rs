use super::*;

#[test]
fn stop_play_logs_preview_stop_for_preview_state() {
    let app = build_test_app();
    let session = app.playback.preview_output.start_session();

    app.stop_play();

    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(!app.playback.preview_output.is_current(session));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn stop_play_logs_play_stop_for_playing_state() {
    let app = build_test_app();
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;

    app.stop_play();

    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}
