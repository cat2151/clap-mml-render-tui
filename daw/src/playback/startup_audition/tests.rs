use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{fold_into_loop, measure_render_settled, StartupAudition};
use crate::{
    playback::tests::build_test_app, CacheState, CellCache, DawApp, DawPlayState,
    FIRST_PLAYABLE_TRACK,
};

fn insert_audition(app: &DawApp, server_settled: bool) {
    *app.playback.startup_audition.lock().unwrap() = Some(StartupAudition::new(
        0,
        Arc::new(AtomicBool::new(server_settled)),
    ));
}

fn audition_state(app: &DawApp) -> Option<bool> {
    app.playback
        .startup_audition
        .lock()
        .unwrap()
        .as_ref()
        .map(|audition| audition.handed_off)
}

fn log_contains(app: &DawApp, needle: &str) -> bool {
    app.log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == needle)
}

fn cell(state: CacheState) -> CellCache {
    let mut cell = CellCache::empty();
    cell.state = state;
    cell
}

#[test]
fn fold_into_loop_wraps_tail_onto_head() {
    assert_eq!(
        fold_into_loop(&[1.0, 2.0, 3.0, 4.0, 0.5, 0.25], 4),
        vec![1.5, 2.25, 3.0, 4.0]
    );
}

#[test]
fn fold_into_loop_pads_short_samples_to_loop_length() {
    assert_eq!(fold_into_loop(&[1.0, 2.0], 4), vec![1.0, 2.0, 0.0, 0.0]);
}

#[test]
fn measure_render_settled_waits_only_for_audible_pending_tracks() {
    let track = FIRST_PLAYABLE_TRACK;
    let mut cache = vec![vec![CellCache::empty(); 2]; track + 2];
    let gains = vec![1.0; track + 2];

    cache[track][1] = cell(CacheState::Rendering);
    assert!(!measure_render_settled(&cache, 1, &gains));

    let mut muted = gains.clone();
    muted[track] = 0.0;
    assert!(measure_render_settled(&cache, 1, &muted));

    cache[track][1] = cell(CacheState::Error);
    cache[track + 1][1] = cell(CacheState::Ready);
    assert!(measure_render_settled(&cache, 1, &gains));
}

#[test]
fn pump_keeps_waiting_until_server_is_ready() {
    let app = build_test_app();
    insert_audition(&app, false);

    app.pump_startup_audition();

    assert_eq!(audition_state(&app), Some(false));
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
}

#[test]
fn pump_hands_off_once_server_and_measure_are_ready() {
    let app = build_test_app();
    insert_audition(&app, true);

    app.pump_startup_audition();
    assert!(log_contains(&app, "startup-audition: hand off meas1"));
    assert_eq!(audition_state(&app), Some(true));

    // ループを鳴らしていないので、次の tick で片付く。
    app.pump_startup_audition();
    assert_eq!(audition_state(&app), None);
}

#[test]
fn pump_does_not_hand_off_while_measure_is_rendering() {
    let app = build_test_app();
    app.cache.lock().unwrap()[FIRST_PLAYABLE_TRACK][1] = cell(CacheState::Pending);
    insert_audition(&app, true);

    app.pump_startup_audition();

    assert_eq!(audition_state(&app), Some(false));
}

#[test]
fn pump_yields_to_playback_started_elsewhere() {
    let app = build_test_app();
    insert_audition(&app, false);
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;

    app.pump_startup_audition();

    assert_eq!(audition_state(&app), None);
    assert!(log_contains(
        &app,
        "startup-audition: cancel reason=other-playback"
    ));
}

#[test]
fn shift_space_while_waiting_stops_audition_without_starting_play() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 1;
    insert_audition(&app, false);

    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

    assert_eq!(audition_state(&app), None);
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
    assert!(log_contains(&app, "startup-audition: cancel reason=user"));
}

#[test]
fn stop_play_cancels_audition() {
    let app = build_test_app();
    insert_audition(&app, false);

    app.stop_play();

    assert_eq!(audition_state(&app), None);
}

#[test]
fn dropping_audition_stops_its_looper() {
    let control = Arc::new(super::LooperControl::default());
    let mut audition = StartupAudition::new(0, Arc::new(AtomicBool::new(false)));
    audition.looper = Some(Arc::clone(&control));

    drop(audition);

    assert!(control.stop.load(Ordering::Acquire));
}
