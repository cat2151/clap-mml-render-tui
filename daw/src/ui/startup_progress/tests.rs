use std::time::Duration;

use cmrt_tui_core::startup_progress::startup_progress_lines;

use super::*;

fn rendered(stage: DawPlaybackStartupStage, server_progress: Option<(usize, usize)>) -> String {
    startup_progress_lines(
        &startup_steps(stage, server_progress),
        Duration::from_millis(1_700),
    )
    .iter()
    .map(|line| {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn the_server_stage_shows_how_many_clap_instances_are_up() {
    let text = rendered(
        DawPlaybackStartupStage::PlayServer {
            first_measure_follows: true,
        },
        Some((9, 14)),
    );

    assert!(text.contains("play server 起動"), "{text}");
    assert!(text.contains("9/14"), "{text}");
    // 次に来る段階も並べておく。何段階待つのかが最初から見えるように。
    assert!(text.contains("1小節目の音色ロード"), "{text}");
}

/// 小節ごとに SMF を投げる `PlayServer` backend はキャッシュを載せない。
/// 起きない段階を並べると「まだ何か待っている」に読める。
#[test]
fn the_legacy_backend_does_not_advertise_a_cache_load_stage() {
    let text = rendered(
        DawPlaybackStartupStage::PlayServer {
            first_measure_follows: false,
        },
        Some((9, 14)),
    );

    assert!(text.contains("play server 起動"), "{text}");
    assert!(!text.contains("1小節目の音色ロード"), "{text}");
}

#[test]
fn the_measure_stage_marks_the_server_stage_as_finished() {
    let text = rendered(
        DawPlaybackStartupStage::FirstMeasure {
            loaded: 3,
            total: 7,
        },
        Some((14, 14)),
    );

    assert!(text.contains("✓ play server 起動"), "{text}");
    assert!(text.contains("▶ 1小節目の音色ロード"), "{text}");
    assert!(text.contains("3/7"), "{text}");
}

/// supervisor がまだ子プロセスを spawn していないあいだ（進捗が 1 行も
/// 出ていない）でも、「動き出している」ことは出す。
#[test]
fn the_server_stage_without_a_report_still_shows_that_it_is_running() {
    let text = rendered(
        DawPlaybackStartupStage::PlayServer {
            first_measure_follows: true,
        },
        None,
    );

    assert!(text.contains("▶ play server 起動"), "{text}");
    assert!(text.contains('…'), "{text}");
}
