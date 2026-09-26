use crate::{
    playback::tests::build_test_app, CacheState, CellCache, DawApp, DawMode, DawPlayState,
    FIRST_PLAYABLE_TRACK,
};

fn reservation(app: &DawApp) -> Option<usize> {
    *app.playback.auto_play_reservation.lock().unwrap()
}

fn log_contains(app: &DawApp, needle: &str) -> bool {
    app.log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == needle)
}

fn set_cell_state(app: &DawApp, measure: usize, state: CacheState) {
    let mut cell = CellCache::empty();
    cell.state = state;
    app.cache.lock().unwrap()[FIRST_PLAYABLE_TRACK][measure] = cell;
}

#[test]
fn reserve_uses_the_cursor_measure_and_the_init_column_maps_to_meas1() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 2;
    app.reserve_auto_play_after_render();
    assert_eq!(reservation(&app), Some(2));

    app.editor.cursor_measure = 0;
    app.reserve_auto_play_after_render();
    assert_eq!(reservation(&app), Some(1));
}

#[test]
fn pump_waits_while_the_reserved_measure_is_rendering() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 1;
    set_cell_state(&app, 1, CacheState::Rendering);
    app.reserve_auto_play_after_render();

    app.pump_auto_play_reservation();

    assert_eq!(reservation(&app), Some(1));
    assert!(!log_contains(&app, "auto-play: start meas1 rendered"));
}

#[test]
fn pump_starts_play_once_the_measure_is_rendered_and_everything_is_silent() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 1;
    set_cell_state(&app, 1, CacheState::Pending);
    app.reserve_auto_play_after_render();
    app.pump_auto_play_reservation();

    set_cell_state(&app, 1, CacheState::Ready);
    app.pump_auto_play_reservation();

    assert_eq!(reservation(&app), None);
    assert!(log_contains(&app, "auto-play: start meas1 rendered"));
}

#[test]
fn pump_skips_when_something_is_already_sounding() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 1;
    app.reserve_auto_play_after_render();
    *app.playback.play_state.lock().unwrap() = DawPlayState::Preview;

    app.pump_auto_play_reservation();

    assert_eq!(reservation(&app), None);
    assert!(log_contains(&app, "auto-play: skip reason=sounding"));
    assert!(!log_contains(&app, "auto-play: start meas1 rendered"));
}

#[test]
fn pump_skips_outside_normal_mode() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 1;
    app.reserve_auto_play_after_render();
    app.mode = DawMode::MmlOverlay;

    app.pump_auto_play_reservation();

    assert_eq!(reservation(&app), None);
    assert!(log_contains(&app, "auto-play: skip reason=not-normal-mode"));
}

#[test]
fn stop_play_cancels_the_reservation() {
    let mut app = build_test_app();
    app.editor.cursor_measure = 1;
    app.reserve_auto_play_after_render();

    app.stop_play();

    assert_eq!(reservation(&app), None);
}
