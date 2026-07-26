use super::*;

#[test]
fn begin_connecting_clears_meter_and_sets_phase() {
    let mut status = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        last_send: Some(Duration::from_millis(2)),
        limiter_reduction_db: 4.0,
        buffer_multiplier: 8,
        underrun_frames: 512,
        server_startup: None,
        patch_setting: None,
    };
    status.begin_connecting();
    assert_eq!(status.phase, GridConnectionPhase::Connecting);
    assert_eq!(status.last_send, None);
    assert_eq!(status.limiter_reduction_db, 0.0);
    assert_eq!(status.buffer_multiplier, 2);
    assert_eq!(status.underrun_frames, 0);
}

#[test]
fn labels_include_server_and_patch_progress() {
    let mut status = GridConnectionStatus::default();
    status.begin_connecting();
    status.update_server_startup(6, 16);
    assert_eq!(status.label(), "starting server 6/16");

    status.begin_patch_setting(16);
    status.update_patch_setting(11, 16);
    assert_eq!(status.label(), "patches 11/16");
}

#[test]
fn periodic_meter_update_uses_the_larger_reduction() {
    let mut status = GridConnectionStatus::default();
    status.update_limiter_meter(cmrt_realtime_play::LimiterMeter {
        current_reduction_db: 1.5,
        peak_reduction_db: 3.0,
    });
    assert_eq!(status.limiter_reduction_db, 3.0);
}

#[test]
fn adaptive_buffer_status_tracks_the_current_level() {
    let mut status = GridConnectionStatus::default();
    status.update_adaptive_buffer(16, 1_024);
    assert_eq!(status.buffer_multiplier, 16);
    assert_eq!(status.underrun_frames, 1_024);
}
