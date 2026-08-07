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
        stage_started_at: None,
        server_startup_elapsed: Some(Duration::from_secs(8)),
        patch_setting_elapsed: Some(Duration::from_millis(300)),
        ..GridConnectionStatus::default()
    };
    status.begin_connecting();
    assert_eq!(status.phase, GridConnectionPhase::Connecting);
    assert_eq!(status.last_send, None);
    assert_eq!(status.limiter_reduction_db, 0.0);
    assert_eq!(status.buffer_multiplier, 2);
    assert_eq!(status.underrun_frames, 0);
    assert_eq!(status.server_startup_elapsed, None);
    assert_eq!(status.patch_setting_elapsed, None);
    assert!(status.stage_started_at.is_some());
}

#[test]
fn labels_include_server_and_patch_progress() {
    let mut status = GridConnectionStatus::default();
    status.begin_connecting();
    status.update_server_startup(6, 16);
    assert_eq!(status.label(), "starting server 6/16");

    status.begin_patch_setting(16, Duration::from_millis(1));
    status.update_patch_setting(11, 16);
    assert_eq!(status.label(), "patches 11/16");
}

#[test]
fn row_readiness_follows_the_two_preparation_stages() {
    let mut status = GridConnectionStatus::default();
    // 進捗中ではない Idle は、テストモードと画面離脱後の状態なので通常表示。
    assert_eq!(status.row_readiness(0), GridRowReadiness::Prepared);
    assert!(!status.is_preparing());

    status.begin_connecting();
    assert_eq!(status.row_readiness(0), GridRowReadiness::Pending);
    status.update_server_startup(3, 16);
    assert_eq!(status.row_readiness(2), GridRowReadiness::InstanceReady);
    assert_eq!(status.row_readiness(3), GridRowReadiness::Pending);
    assert!(status.is_preparing());

    status.wait_for_patches(Duration::from_millis(1));
    assert_eq!(status.row_readiness(15), GridRowReadiness::InstanceReady);

    status.begin_patch_setting(16, Duration::from_millis(1));
    assert_eq!(status.row_readiness(0), GridRowReadiness::InstanceReady);
    status.update_patch_setting(5, 16);
    assert_eq!(status.row_readiness(4), GridRowReadiness::Prepared);
    assert_eq!(status.row_readiness(5), GridRowReadiness::InstanceReady);

    status.phase = GridConnectionPhase::Ready;
    assert_eq!(status.row_readiness(15), GridRowReadiness::Prepared);
    assert!(!status.is_preparing());
}

#[test]
fn an_error_greys_out_every_row_and_exposes_its_message() {
    let status = GridConnectionStatus {
        phase: GridConnectionPhase::Error("patch prepare failed".to_string()),
        ..GridConnectionStatus::default()
    };

    assert_eq!(status.row_readiness(0), GridRowReadiness::Pending);
    assert_eq!(status.row_readiness(15), GridRowReadiness::Pending);
    assert_eq!(status.error_message(), Some("patch prepare failed"));
    assert!(!status.is_preparing());
    assert_eq!(GridConnectionStatus::default().error_message(), None);
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

#[test]
fn one_row_patch_load_does_not_stop_other_rows() {
    let mut status = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        ..GridConnectionStatus::default()
    };

    status.begin_row_patch_setting(2);

    assert!(status.row_patch_is_loading());
    assert_eq!(status.row_readiness(2), GridRowReadiness::InstanceReady);
    assert_eq!(status.row_readiness(1), GridRowReadiness::Prepared);
    assert!(status.phase.accepts_notes());
    assert_eq!(status.label(), "ready | instance 3 patch loading");

    status.finish_row_patch_setting(2, None);
    assert!(!status.row_patch_is_loading());
    assert_eq!(status.row_readiness(2), GridRowReadiness::Prepared);
}

#[test]
fn one_row_patch_error_is_visible_without_disabling_playback() {
    let mut status = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        ..GridConnectionStatus::default()
    };

    status.begin_row_patch_setting(1);
    status.finish_row_patch_setting(1, Some("bad patch".to_string()));

    assert_eq!(status.row_readiness(1), GridRowReadiness::Pending);
    assert_eq!(status.row_readiness(0), GridRowReadiness::Prepared);
    assert!(status.phase.accepts_notes());
    assert_eq!(status.label(), "ready | instance 2 patch error: bad patch");
}
