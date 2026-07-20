use super::*;

fn clip(name: &str, span_measures: usize, bpm: f64) -> Option<LoopPlaybackClip> {
    Some(LoopPlaybackClip {
        path: PathBuf::from(name),
        span_measures,
        bpm,
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
    let target = grid_target_bpm(&grid);
    assert_eq!(target.bpm, 120.0);
    assert_eq!(
        measure_duration(&grid, target.bpm),
        Duration::from_millis(2_000)
    );
}

#[test]
fn measure_duration_uses_the_automatically_adjusted_bpm() {
    let grid = vec![vec![clip("fast.wav", 1, 160.0)]];
    let target = grid_target_bpm(&grid);

    assert_eq!(target.bpm, 128.0);
    assert_eq!(
        measure_duration(&grid, target.bpm),
        Duration::from_millis(1_875)
    );
}

#[test]
fn incompatible_clips_fall_back_to_120() {
    let grid = vec![vec![clip("slow.wav", 1, 60.0), clip("fast.wav", 1, 200.0)]];
    let target = grid_target_bpm(&grid);

    assert_eq!(target.bpm, 120.0);
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
