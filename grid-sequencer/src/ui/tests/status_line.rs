//! 最下部のステータス行。狭い端末でも patch 状態まで出し切れることを確かめる。

use super::*;

#[test]
fn the_status_line_shows_instances_and_limiter_reduction() {
    let mut screen = screen_with_first_row(60, &[]);
    screen.patch_status = GridPatchStatus::Ready(42);

    let rendered = render(&screen);

    assert!(rendered.contains("SHM idle"), "{rendered}");
    assert!(rendered.contains("130bpm"), "{rendered}");
    assert!(rendered.contains("16i/16l"), "{rendered}");
    assert!(rendered.contains("s1/16"), "{rendered}");
    assert!(rendered.contains("GR0.0"), "{rendered}");
    assert!(rendered.contains("p:42"), "{rendered}");
}

#[test]
fn the_status_line_shows_adaptive_buffer_and_current_level_underruns() {
    let screen = screen_with_first_row(60, &[]);
    let connection = GridConnectionStatus {
        buffer_multiplier: 8,
        underrun_frames: 1_536,
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("x8/85ms"), "{rendered}");
    assert!(rendered.contains("d1536/0"), "{rendered}");
}

/// 倍率が上がるほど想定レイテンシも伸びる。x256 まで出し切っても桁が溢れないこと。
#[test]
fn the_status_line_shows_the_expected_latency_of_the_largest_buffer() {
    let screen = screen_with_first_row(60, &[]);
    let connection = GridConnectionStatus {
        buffer_multiplier: 256,
        underrun_frames: 1_536,
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("x256/2731ms"), "{rendered}");
    assert!(rendered.contains("p:"), "{rendered}");
}

#[test]
fn the_status_line_shows_instance_startup_progress() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Connecting,
        server_startup: Some(GridProgress {
            completed: 6,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("SHM starting server 6/16"), "{rendered}");
}

#[test]
fn the_status_line_shows_patch_setting_progress() {
    let screen = GridSequencerScreen::new(None);
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::PatchSetting,
        patch_setting: Some(GridProgress {
            completed: 11,
            total: 16,
        }),
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);

    assert!(rendered.contains("SHM patches 11/16"), "{rendered}");
}

#[test]
fn the_status_line_exposes_timing_even_when_drop_is_zero() {
    let screen = screen_with_first_row(60, &[]);
    let connection = GridConnectionStatus {
        underrun_frames: 0,
        underrun_frames_total: 24,
        timing: cmrt_realtime_play::TimingMetrics {
            late_events: 3,
            max_late_us: 750.0,
            output_lead_min_frames: 480,
            output_lead_max_frames: 960,
            process_load_p95: 37.0,
            ..cmrt_realtime_play::TimingMetrics::default()
        },
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);
    assert!(rendered.contains("d0/24"), "{rendered}");
    assert!(rendered.contains("l3/750u"), "{rendered}");
    assert!(rendered.contains("lead10-20"), "{rendered}");
    assert!(rendered.contains("p37%"), "{rendered}");
}
