use super::*;

use crate::CHORD_TRACK;
use crate::FIRST_PLAYABLE_TRACK;

/// crossterm は Shift 付きの大文字として届けるので、テストもその形で送る。
fn press_c(app: &mut DawApp) {
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
}

#[test]
fn c_jumps_to_the_chord_row_without_moving_the_measure() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK + 1;
    app.editor.cursor_measure = 2;

    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, CHORD_TRACK);
    assert_eq!(app.editor.cursor_measure, 2);
}

#[test]
fn pressing_c_again_returns_to_the_track_it_came_from() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK + 1;
    app.editor.cursor_measure = 2;

    press_c(&mut app);
    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK + 1);
    assert_eq!(app.editor.cursor_measure, 2);
}

#[test]
fn the_return_target_is_forgotten_after_it_is_used_once() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK + 1;

    press_c(&mut app);
    press_c(&mut app);

    assert_eq!(app.editor.chord_jump_return_track, None);

    // 戻ったあとに手で chord 行へ降りると、戻り先はもう覚えていないので
    // 最初の演奏 track が使われる。
    app.editor.cursor_track = CHORD_TRACK;
    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
}

#[test]
fn c_from_the_chord_row_without_a_return_target_goes_to_the_first_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = CHORD_TRACK;
    app.editor.cursor_measure = 1;

    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
    assert_eq!(app.editor.cursor_measure, 1);
}

#[test]
fn a_return_target_that_no_longer_exists_falls_back_to_the_first_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = CHORD_TRACK;
    // project を開き直して track が減った、のような状況。
    app.editor.chord_jump_return_track = Some(app.editor.tracks + 5);

    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
}

#[test]
fn the_tempo_row_is_never_used_as_a_return_target() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = crate::tracks::TEMPO_TRACK;

    press_c(&mut app);
    assert_eq!(app.editor.cursor_track, CHORD_TRACK);

    press_c(&mut app);

    // Tempo 行から跳んでも、戻り先は演奏 track。
    // Tempo 行へ戻しても preview もできず、行き止まりになるため。
    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
}

#[test]
fn jumping_to_the_chord_row_stops_the_preview_and_coming_back_restarts_it() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    app.editor.cursor_measure = 1;
    app.editor.data[FIRST_PLAYABLE_TRACK][1] = "cdef".to_string();
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
        measure_duration: std::time::Duration::from_secs(1),
    });

    press_c(&mut app);

    // chord 行は演奏されないので preview の対象にならない。
    assert_eq!(app.editor.cursor_track, CHORD_TRACK);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));

    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
}

#[test]
fn c_does_not_stop_playback_that_is_already_running() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;

    press_c(&mut app);

    assert_eq!(app.editor.cursor_track, CHORD_TRACK);
    assert!(matches!(
        *app.playback.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
}

#[test]
fn c_does_not_leave_a_pending_delete_armed() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    app.handle_normal(KeyCode::Char('d'));
    assert!(app.editor.pending_delete);

    press_c(&mut app);

    assert!(!app.editor.pending_delete);
    assert_eq!(app.editor.cursor_track, CHORD_TRACK);
}
