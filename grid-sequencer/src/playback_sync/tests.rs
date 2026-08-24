use super::*;

#[test]
fn half_measure_drop_is_removed_from_the_display_clock() {
    let started = Instant::now();
    let mut sync = PlaybackSync::default();
    sync.restart(started, 10_000);

    let half_measure = crate::step_offset(8);
    let dropped_frames = (half_measure.as_secs_f64() * 48_000.0).round() as u64;
    let wall_now = started + crate::step_offset(24);
    let audible = sync.audible_now(wall_now, 10_000 + dropped_frames, 48_000.0);
    let expected = started + crate::step_offset(16);

    let error = if audible >= expected {
        audible - expected
    } else {
        expected - audible
    };
    assert!(
        error <= Duration::from_secs_f64(1.0 / 48_000.0),
        "audible={audible:?} expected={expected:?}"
    );
}

#[test]
fn a_delayed_counter_update_freezes_instead_of_rewinding_the_display() {
    let started = Instant::now();
    let mut sync = PlaybackSync::default();
    sync.restart(started, 0);
    let before_drop = sync.audible_now(started + Duration::from_secs(1), 0, 48_000.0);

    let after_report = sync.audible_now(started + Duration::from_millis(1_010), 24_000, 48_000.0);

    assert_eq!(after_report, before_drop);
}

#[test]
fn the_sync_check_runs_once_for_each_displayed_measure() {
    let started = Instant::now();
    let mut sync = PlaybackSync::default();
    sync.restart(started, 0);
    sync.audible_now(started, 480, 48_000.0);

    let first = sync
        .check_measure(Some(0), Duration::from_millis(2), 48_000.0)
        .unwrap();
    assert_eq!(first.measure, 0);
    assert_eq!(first.dropped_frames, 480);
    assert_eq!(first.dropped, Duration::from_millis(10));
    assert!(sync
        .check_measure(Some(15), Duration::ZERO, 48_000.0)
        .is_none());

    let second = sync
        .check_measure(Some(16), Duration::from_millis(3), 48_000.0)
        .unwrap();
    assert_eq!(second.measure, 1);
    assert_eq!(second.display_lateness, Duration::from_millis(3));
}

#[test]
fn a_server_counter_reset_does_not_invent_an_enormous_drop() {
    let started = Instant::now();
    let mut sync = PlaybackSync::default();
    sync.restart(started, 50_000);

    let audible = sync.audible_now(started + Duration::from_secs(1), 12, 48_000.0);

    assert_eq!(audible, started + Duration::from_secs(1));
}
