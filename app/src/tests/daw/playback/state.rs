use super::*;

#[test]
fn stop_play_logs_preview_stop_for_preview_state() {
    let app = build_test_app();
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    let initial_session = app
        .preview_session
        .load(std::sync::atomic::Ordering::Acquire);

    app.stop_play();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        app.preview_session
            .load(std::sync::atomic::Ordering::Acquire),
        initial_session + 1
    );
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn stop_play_logs_play_stop_for_playing_state() {
    let app = build_test_app();
    *app.play_state.lock().unwrap() = DawPlayState::Playing;

    app.stop_play();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}
