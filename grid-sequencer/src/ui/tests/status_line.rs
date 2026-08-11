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
    assert!(rendered.contains("step 1/16"), "{rendered}");
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

    assert!(rendered.contains("buf x8 85ms"), "{rendered}");
    assert!(rendered.contains("underrun 1536f"), "{rendered}");
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

    assert!(rendered.contains("buf x256 2731ms"), "{rendered}");
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
