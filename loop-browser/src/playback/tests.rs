use super::*;
use std::collections::HashMap;
use std::time::Duration;

fn clip(name: &str, span_measures: usize, bpm: f64) -> Option<LoopPlaybackClip> {
    Some(LoopPlaybackClip {
        path: PathBuf::from(name),
        span_measures,
        kind: cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
        bpm: Some(bpm),
        category: None,
        meter_numerator: 4,
        meter_denominator: 4,
    })
}

fn one_shot(name: &str) -> Option<LoopPlaybackClip> {
    Some(LoopPlaybackClip {
        path: PathBuf::from(name),
        span_measures: 1,
        kind: cmrt_loop_domain::loop_wav_analysis::LoopWavKind::OneShot,
        bpm: None,
        category: None,
        meter_numerator: 4,
        meter_denominator: 4,
    })
}

#[test]
fn next_measure_skips_empty_columns_and_wraps() {
    let grid = vec![
        vec![clip("a.wav", 1, 120.0), None, clip("c.wav", 1, 120.0)],
        vec![clip("b.wav", 1, 90.0), None, None],
    ];
    assert_eq!(next_measure(&grid, None), Some(0));
    assert_eq!(next_measure(&grid, Some(0)), Some(2));
    assert_eq!(next_measure(&grid, Some(2)), Some(0));
    assert_eq!(
        starting_clips(&grid, 0)
            .into_iter()
            .map(|(_, clip)| clip.path.clone())
            .collect::<Vec<_>>(),
        vec![PathBuf::from("a.wav"), PathBuf::from("b.wav")]
    );
}

#[test]
fn next_measure_returns_none_for_empty_grid() {
    assert_eq!(next_measure(&vec![vec![None, None]], None), None);
}

#[test]
fn tempo_stays_at_120_when_all_clips_fit_and_uses_the_first_clip_meter() {
    let grid = vec![
        vec![None, clip("top.wav", 1, 120.0)],
        vec![clip("first.wav", 2, 100.0), None],
    ];
    let target = grid_target_bpm(
        &grid,
        cmrt_tui_core::bpm::BpmMode::Auto(cmrt_loop_browser_domain::time_stretch::TARGET_BPM),
    );
    assert_eq!(target.bpm, 120.0);
    assert_eq!(
        measure_duration(&grid, target.bpm),
        Duration::from_millis(2_000)
    );
}

#[test]
fn measure_duration_uses_the_automatically_adjusted_bpm() {
    let grid = vec![vec![clip("fast.wav", 1, 160.0)]];
    let target = grid_target_bpm(
        &grid,
        cmrt_tui_core::bpm::BpmMode::Auto(cmrt_loop_browser_domain::time_stretch::TARGET_BPM),
    );

    assert_eq!(target.bpm, 128.0);
    assert_eq!(
        measure_duration(&grid, target.bpm),
        Duration::from_millis(1_875)
    );
}

#[test]
fn one_shots_do_not_affect_target_bpm_or_meter_selection() {
    let mut three_four = clip("loop.wav", 1, 160.0).unwrap();
    three_four.meter_numerator = 3;
    let grid = vec![
        vec![one_shot("hit.wav"), None],
        vec![None, Some(three_four)],
    ];
    let target = grid_target_bpm(
        &grid,
        cmrt_tui_core::bpm::BpmMode::Auto(cmrt_loop_browser_domain::time_stretch::TARGET_BPM),
    );
    let timing = measure_timing(&grid, target.bpm);

    assert_eq!(target.bpm, 128.0);
    assert_eq!(timing.beats_per_measure, 3);
}

#[test]
fn all_one_shots_use_default_tempo_without_a_stretch_constraint() {
    let grid = vec![vec![one_shot("hit.wav")]];
    let target = grid_target_bpm(
        &grid,
        cmrt_tui_core::bpm::BpmMode::Auto(cmrt_loop_browser_domain::time_stretch::TARGET_BPM),
    );

    assert_eq!(target.bpm, 120.0);
    assert!(target.has_common_range);
    assert_eq!(measure_duration(&grid, target.bpm), Duration::from_secs(2));
}

#[test]
fn measure_timing_exposes_the_meter_numerator_as_beat_count() {
    let mut three_four = clip("three-four.wav", 1, 120.0).unwrap();
    three_four.meter_numerator = 3;
    let timing = measure_timing(&vec![vec![Some(three_four)]], 120.0);

    assert_eq!(timing.duration, Duration::from_millis(1_500));
    assert_eq!(timing.beats_per_measure, 3);
}

#[test]
fn incompatible_clips_fall_back_to_120() {
    let grid = vec![vec![clip("slow.wav", 1, 60.0), clip("fast.wav", 1, 200.0)]];
    let target = grid_target_bpm(
        &grid,
        cmrt_tui_core::bpm::BpmMode::Auto(cmrt_loop_browser_domain::time_stretch::TARGET_BPM),
    );

    assert_eq!(target.bpm, 120.0);
    assert!(!target.has_common_range);
}

#[test]
fn manual_bpm_overrides_automatic_selection_without_clamping() {
    let grid = vec![vec![clip("fast.wav", 1, 160.0)]];
    let target = grid_target_bpm(&grid, cmrt_tui_core::bpm::BpmMode::Manual(90.123456789));

    assert_eq!(target.bpm, 90.123456789);
    assert!(!target.has_common_range);
}

#[test]
fn continuation_measure_waits_and_can_start_another_track() {
    let grid = vec![
        vec![clip("long.wav", 2, 120.0), None],
        vec![None, clip("next.wav", 1, 120.0)],
    ];
    assert_eq!(next_measure(&grid, None), Some(0));
    assert_eq!(next_measure(&grid, Some(0)), Some(1));
    assert_eq!(
        starting_clips(&grid, 1)
            .into_iter()
            .map(|(_, clip)| clip.path.clone())
            .collect::<Vec<_>>(),
        vec![PathBuf::from("next.wav")]
    );
}

#[test]
fn scheduled_output_is_cropped_to_the_prev_markers_effective_span() {
    let one_measure = Duration::from_secs(2);

    assert_eq!(
        scheduling::scheduled_output_frames(176_400, 44_100, one_measure),
        88_200
    );
    assert_eq!(
        scheduling::scheduled_output_frames(88_200, 44_100, one_measure),
        88_200
    );
}

#[test]
fn one_shot_output_is_not_cropped_to_the_measure_duration() {
    let hit = one_shot("hit.wav").unwrap();

    assert_eq!(
        scheduling::scheduled_clip_output_frames(&hit, 398_224, 44_100, Duration::from_secs(2)),
        398_224
    );
}

#[test]
fn scheduled_idle_reports_the_gap_between_one_shot_end_and_span_boundary() {
    let idle = scheduling::scheduled_idle_seconds(211_028, 44_100, Duration::from_secs(8));

    assert!((idle - 3.214_785).abs() < 0.000_001);
}

#[test]
fn updated_grid_is_used_when_the_next_measure_is_selected() {
    let before = vec![vec![
        clip("current.wav", 1, 120.0),
        None,
        clip("old.wav", 1, 120.0),
    ]];
    assert_eq!(next_measure(&before, Some(0)), Some(2));

    let after = vec![vec![
        clip("current.wav", 1, 120.0),
        clip("new.wav", 1, 120.0),
        None,
    ]];
    assert_eq!(next_measure(&after, Some(0)), Some(1));
}

#[test]
fn restart_measure_is_selected_immediately_instead_of_the_following_measure() {
    let grid = vec![vec![
        clip("first.wav", 1, 120.0),
        clip("replaced.wav", 1, 120.0),
        clip("following.wav", 1, 120.0),
    ]];

    assert_eq!(measure_at_or_after(&grid, 1), Some(1));
    assert_eq!(next_measure(&grid, Some(1)), Some(2));
}

#[test]
fn restart_measure_skips_forward_over_an_empty_requested_column() {
    let grid = vec![vec![
        clip("first.wav", 1, 120.0),
        None,
        clip("next.wav", 1, 120.0),
    ]];

    assert_eq!(measure_at_or_after(&grid, 1), Some(2));
}

#[test]
fn pad_voices_replace_only_the_same_pad() {
    let mut voices = HashMap::from([('c', "first-c"), ('d', "first-d")]);

    assert_eq!(take_pad_voice(&mut voices, 'c'), Some("first-c"));
    voices.insert('c', "second-c");
    assert_eq!(voices.get(&'c'), Some(&"second-c"));
    assert_eq!(voices.get(&'d'), Some(&"first-d"));
}

#[test]
fn paused_transport_stays_stopped_across_grid_updates_and_resumes_at_selection() {
    let first_grid = vec![vec![
        clip("first.wav", 1, 120.0),
        clip("second.wav", 1, 120.0),
    ]];
    let updated_grid = vec![vec![
        clip("new-first.wav", 1, 120.0),
        None,
        clip("new-third.wav", 1, 120.0),
    ]];
    let mut transport = TransportState::default();

    assert_eq!(transport.next_measure_to_start(&first_grid), Some(0));
    transport.pause();
    assert!(transport.is_paused());
    assert_eq!(transport.next_measure_to_start(&updated_grid), None);

    transport.resume_at(1);
    assert!(!transport.is_paused());
    assert_eq!(transport.next_measure_to_start(&updated_grid), Some(2));
}

#[test]
fn effective_gain_combines_solo_state_with_saved_track_volume() {
    let volumes = [3, -6, 0];
    let expected_solo_gain = cmrt_tui_core::mixer::volume_db_to_gain(-6);

    assert_eq!(
        gain::effective_track_gain(0, &volumes, &[false, false, false]),
        cmrt_tui_core::mixer::volume_db_to_gain(3)
    );
    assert_eq!(
        gain::effective_track_gain(0, &volumes, &[false, true, false]),
        0.0
    );
    assert_eq!(
        gain::effective_track_gain(1, &volumes, &[false, true, false]),
        expected_solo_gain
    );
    assert_eq!(gain::effective_track_gain(2, &volumes, &[false, true]), 0.0);
}
