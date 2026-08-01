use super::*;

#[test]
fn persisted_analysis_drives_browser_without_opening_the_wav() {
    let analysis = LoopWavAnalysis {
        duration_seconds: 5.25,
        kind: LoopWavKind::Loop,
        tempo: Some(LoopTempoAnalysis {
            bpm: 137.5,
            declared_bpm: Some(137.5),
            beats: 9,
            meter_numerator: 3,
            meter_denominator: 4,
            source: LoopAnalysisSource::Acid,
        }),
        measures: 3,
    };
    let mut browser = LoopBrowser::from_index(
        LoopIndex {
            version: cmrt_loop_browser_domain::library::LOOP_INDEX_VERSION,
            roots: vec![LoopRootIndex {
                path: "/path/that/does/not/exist".to_string(),
                wav_files: vec![LoopWavIndex {
                    relative: "cached.wav".to_string(),
                    analysis,
                    waveform: cmrt_loop_domain::loop_waveform::LoopWaveform::silent(
                        analysis.measures,
                    ),
                }],
            }],
        },
        &cmrt_runtime::default_loop_categories(),
        cmrt_loop_browser_domain::persisted::PersistedDoc::in_memory(LoopBrowserMetadata::default()),
    );
    let wav = browser.wav_analyses[0].0.clone();
    browser.metadata.value.toggle_pad('c', &wav);
    browser.focus = LoopBrowserPane::Tracks;

    browser.handle_key(KeyCode::Char('c'));
    let grid = browser.playback_grid();
    let clip = grid[0][0].as_ref().unwrap();

    assert_eq!(browser.track_grid[0][0].as_ref().unwrap().span_measures, 3);
    assert_eq!(clip.bpm, Some(137.5));
    assert_eq!(clip.meter_numerator, 3);
    assert_eq!(clip.meter_denominator, 4);
}

#[test]
fn target_bpm_tracks_all_placed_wavs_and_returns_to_120_after_removal() {
    let mut browser = browser_with_direct_wavs(2);
    browser.wav_analyses[0].1.tempo.as_mut().unwrap().bpm = 160.0;
    browser.wav_analyses[1].1.tempo.as_mut().unwrap().bpm = 120.0;
    let fast = browser.wav_analyses[0].0.clone();
    let normal = browser.wav_analyses[1].0.clone();
    browser.track_grid[0] = vec![
        Some(LoopTrackClip::explicit(fast, 1)),
        Some(LoopTrackClip::explicit(normal, 1)),
    ];

    assert_eq!(browser.target_bpm().bpm, 128.0);

    browser.track_grid[0][0] = None;
    assert_eq!(browser.target_bpm().bpm, 120.0);
}

#[test]
fn one_shot_analysis_is_excluded_from_target_and_stretch_limit_checks() {
    let mut browser = browser_with_direct_wavs(2);
    browser.wav_analyses[0].1 = browser.wav_analyses[0].1.into_one_shot();
    browser.wav_analyses[1].1.tempo.as_mut().unwrap().bpm = 160.0;
    let hit = LoopTrackClip::explicit(browser.wav_analyses[0].0.clone(), 1);
    let loop_clip = LoopTrackClip::explicit(browser.wav_analyses[1].0.clone(), 1);
    browser.track_grid[0] = vec![Some(hit.clone()), Some(loop_clip)];

    assert_eq!(browser.target_bpm().bpm, 128.0);
    assert_eq!(browser.playback_clip(&hit).bpm, None);
    assert!(!browser.clip_exceeds_time_ratio_limits(&hit, 37.0));
}

#[test]
fn stretch_limit_display_check_uses_the_selected_target_bpm() {
    let mut browser = browser_with_direct_wavs(2);
    browser.wav_analyses[0].1.tempo.as_mut().unwrap().bpm = 100.0;
    browser.wav_analyses[1].1.tempo.as_mut().unwrap().bpm = 200.0;
    let compatible = LoopTrackClip::explicit(browser.wav_analyses[0].0.clone(), 1);
    let rejected = LoopTrackClip::explicit(browser.wav_analyses[1].0.clone(), 1);
    browser.track_grid = vec![vec![Some(compatible.clone()), Some(rejected.clone())]];

    let target = browser.target_bpm();

    assert_eq!(target.bpm, 120.0);
    assert!(!target.has_common_range);
    assert!(!browser.clip_exceeds_time_ratio_limits(&compatible, target.bpm));
    assert!(browser.clip_exceeds_time_ratio_limits(&rejected, target.bpm));
}

#[test]
fn track_pane_toggles_one_wav_per_cell_and_auto_extends_right_and_down() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    browser.handle_key(KeyCode::Tab);
    assert_eq!(browser.focus, LoopBrowserPane::Tracks);

    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::GridRefresh { grid, .. }
            if grid[0][0].as_ref().is_some_and(|clip| clip.path.ends_with("a.wav"))
    ));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('j'));
    assert_eq!((browser.track_cursor, browser.measure_cursor), (1, 1));
    assert_eq!(browser.track_grid.len(), 2);
    assert!(browser.track_grid.iter().all(|track| track.len() == 2));

    browser.handle_key(KeyCode::Char('h'));
    browser.handle_key(KeyCode::Char('k'));
    assert_eq!((browser.track_cursor, browser.measure_cursor), (0, 0));
    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::GridRefresh { grid, .. } if grid[0][0].is_none()
    ));
}

#[test]
fn every_pad_key_places_and_removes_without_audition() {
    for pad in PAD_KEYS {
        let mut browser = browser_with_direct_wavs(1);
        let wav = browser.wav_analyses[0].0.clone();
        browser.metadata.value.toggle_pad(pad, &wav);
        browser.focus = LoopBrowserPane::Tracks;

        assert!(matches!(
            browser.handle_key(KeyCode::Char(pad)),
            LoopBrowserAction::GridRefresh { grid, .. } if grid[0][0].is_some()
        ));
        assert!(matches!(
            browser.handle_key(KeyCode::Char(pad)),
            LoopBrowserAction::GridRefresh { grid, .. } if grid[0][0].is_none()
        ));
    }
}

#[test]
fn every_pad_key_replacement_uses_the_clip_start_without_audition() {
    for pad in PAD_KEYS {
        let mut browser = browser_with_spanning_wavs();
        let original = browser.wav_analyses[0].0.clone();
        let replacement = browser.wav_analyses[1].0.clone();
        browser.track_grid[0].resize(2, None);
        browser.track_grid[0][0] = Some(LoopTrackClip::explicit(original, 2));
        browser.metadata.value.toggle_pad(pad, &replacement);
        browser.focus = LoopBrowserPane::Tracks;
        browser.measure_cursor = 1;

        assert!(matches!(
            browser.handle_key(KeyCode::Char(pad)),
            LoopBrowserAction::GridReplaced {
                start_measure: 0,
                grid,
                ..
            } if grid[0][0]
                .as_ref()
                .is_some_and(|clip| clip.path.ends_with("short.wav"))
                && grid[0].len() == 1
        ));
    }
}

#[test]
fn playback_grid_uses_longest_content_end_and_persisted_prev_markers_across_gaps() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    let short = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![
            Some(LoopTrackClip::explicit(short.clone(), 1)),
            None,
            None,
            None,
        ],
        vec![Some(LoopTrackClip::explicit(long, 4)), None, None, None],
        vec![
            Some(LoopTrackClip::explicit(short.clone(), 1)),
            None,
            Some(LoopTrackClip::explicit(short, 1)),
            None,
        ],
    ];
    browser.normalize_track_grid();

    let grid = browser.playback_grid();

    assert!(grid[0].iter().all(Option::is_some));
    assert!(grid[1][0].is_some());
    assert!(grid[1][1..].iter().all(Option::is_none));
    assert!(grid[2].iter().all(Option::is_some));
    assert!(grid[2][3].is_some());
    assert!(browser
        .clip_at(0, 1)
        .is_some_and(|(_, clip)| clip.is_previous()));
    assert!(browser
        .clip_at(1, 2)
        .is_some_and(|(_, clip)| !clip.is_previous()));
    assert!(browser
        .clip_at(2, 1)
        .is_some_and(|(_, clip)| clip.is_previous()));
    assert!(browser
        .clip_at(2, 3)
        .is_some_and(|(_, clip)| clip.is_previous()));
}

#[test]
fn a_one_shot_retriggers_only_at_its_own_power_of_two_boundary() {
    let mut browser = browser_with_spanning_wavs();
    browser.wav_analyses[1].1 = browser.wav_analyses[1].1.into_one_shot();
    // BPM120 4/4 なら 1 小節 2 秒。5 秒鳴る one-shot は 2.5 小節ぶんなので 4 小節へ切り上げる。
    browser.wav_analyses[1].1.duration_seconds = 5.0;
    let long_loop = browser.wav_analyses[0].0.clone();
    let one_shot = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(one_shot, 1))],
        vec![Some(LoopTrackClip::explicit(long_loop, 8))],
    ];
    browser.normalize_track_grid();

    assert_eq!(browser.measure_seconds(), 2.0);
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| !clip.is_previous() && clip.span_measures == 4));

    let playback = browser.playback_grid();
    // 4 小節の区切りでだけ鳴らし直す。長い空白も、鳴り終わる前の重なりも作らない。
    assert!(playback[0][0]
        .as_ref()
        .is_some_and(LoopPlaybackClip::is_one_shot));
    assert!(playback[0][4]
        .as_ref()
        .is_some_and(LoopPlaybackClip::is_one_shot));
    assert!(playback[0][1..4].iter().all(Option::is_none));
    assert!(playback[0][5..].iter().all(Option::is_none));

    // 表示側は鳴り始めた小節を起点に波形を割り付ける（毎小節リプレイして見えないこと）。
    assert!(browser
        .clip_at(0, 2)
        .is_some_and(|(start, clip)| start == 0 && clip.span_measures == 4));
    assert!(browser
        .clip_at(0, 6)
        .is_some_and(|(start, clip)| start == 4 && clip.is_previous()));
}

#[test]
fn a_one_shot_longer_than_the_loops_extends_the_grid_axis() {
    let mut browser = browser_with_spanning_wavs();
    browser.wav_analyses[1].1 = browser.wav_analyses[1].1.into_one_shot();
    browser.wav_analyses[1].1.duration_seconds = 5.0;
    let short_loop = browser.wav_analyses[0].0.clone();
    let one_shot = browser.wav_analyses[1].0.clone();
    browser.wav_analyses[0].1.measures = 1;
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(one_shot, 1))],
        vec![Some(LoopTrackClip::explicit(short_loop, 1))],
    ];
    browser.normalize_track_grid();

    assert_eq!(browser.track_grid[0].len(), 4);
    // 1 小節ループ側は 4 小節ぶん繰り返して埋まる。
    assert!(browser.playback_grid()[1].iter().all(Option::is_some));
}

#[test]
fn the_cycle_seconds_are_the_measure_seconds_times_the_played_measure_count() {
    let mut browser = browser_with_direct_wavs(1);
    let wav = browser.wav_analyses[0].0.clone();

    // BPM120 4/4 なので 1 小節 2 秒。4 小節のグリッドは 1 周 8 秒。
    browser.track_grid[0] = vec![
        Some(LoopTrackClip::explicit(wav, 4)),
        None,
        None,
        None,
        None,
        None,
    ];
    assert_eq!(browser.measure_seconds(), 2.0);
    assert_eq!(browser.cycle_seconds(), 8.0);

    // cursor で伸びるのは表示だけ。鳴る長さは変わらない。
    browser.measure_cursor = 5;
    assert_eq!(browser.displayed_measure_count(), 6);
    assert_eq!(browser.cycle_seconds(), 8.0);

    // 空のグリッドでも 0 や NaN にはしない（既定の 1 小節ぶん）。
    browser.track_grid[0] = vec![None];
    assert_eq!(browser.cycle_seconds(), 2.0);
}

#[test]
fn allocated_empty_columns_do_not_extend_playback_or_normal_display() {
    let mut browser = browser_with_direct_wavs(1);
    let wav = browser.wav_analyses[0].0.clone();
    browser.track_grid[0] = vec![
        Some(LoopTrackClip::explicit(wav, 1)),
        None,
        None,
        None,
        None,
        None,
    ];

    assert_eq!(browser.playback_grid()[0].len(), 1);
    assert_eq!(browser.displayed_measure_count(), 1);
    assert!(browser.clip_at(0, 1).is_none());

    browser.measure_cursor = 5;
    assert_eq!(browser.displayed_measure_count(), 6);
    assert!(browser.clip_at(0, 5).is_none());
}

#[test]
fn track_pad_write_failures_do_not_audition_or_change_the_grid() {
    let mut browser = browser_with_direct_wavs(2);
    let original = browser.wav_analyses[0].0.clone();
    let replacement = browser.wav_analyses[1].0.clone();
    browser.track_grid[0][0] = Some(LoopTrackClip::explicit(original.clone(), 1));
    browser.metadata.value.toggle_pad('c', &replacement);
    browser.focus = LoopBrowserPane::Tracks;

    browser.track_grid_writable = false;
    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::Continue
    ));
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| clip.wav.matches(&original)));

    browser.track_grid_writable = true;
    let blocked = std::env::temp_dir().join(format!(
        "cmrt-loop-pad-blocked-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&blocked).unwrap();
    browser.track_grid_path = Some(blocked.clone());

    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::Continue
    ));
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| clip.wav.matches(&original)));
    assert!(browser
        .track_grid_error
        .as_deref()
        .is_some_and(|error| error.contains("track listを保存できません")));
    let _ = std::fs::remove_dir_all(blocked);
}

#[test]
fn track_hjkl_prefix_moves_and_extends_by_the_requested_count() {
    let mut browser = browser();
    browser.focus = LoopBrowserPane::Tracks;

    browser.handle_key(KeyCode::Char('3'));
    browser.handle_key(KeyCode::Char('l'));
    assert_eq!(browser.measure_cursor, 3);
    assert!(browser.track_grid.iter().all(|track| track.len() == 4));

    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('j'));
    assert_eq!(browser.track_cursor, 2);
    assert_eq!(browser.track_grid.len(), 3);

    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('h'));
    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('k'));
    assert_eq!((browser.track_cursor, browser.measure_cursor), (0, 1));
}

#[test]
fn replacing_pad_does_not_change_existing_track_cell() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    browser.handle_key(KeyCode::Tab);
    browser.handle_key(KeyCode::Char('c'));
    browser.handle_key(KeyCode::Tab);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert!(browser.notice.is_some());
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert!(browser.notice.is_none());
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));

    assert!(browser.notice.is_none());
    assert!(browser
        .metadata
        .value
        .pad('c')
        .is_some_and(|wav| wav.path().ends_with("B.wav")));
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| clip.wav.path().ends_with("a.wav")));
}

#[test]
fn spanning_clip_occupies_continuations_and_overlap_replaces_whole_clip() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    let short = browser.wav_analyses[1].0.clone();
    browser.metadata.value.toggle_pad('c', &long);
    browser.metadata.value.toggle_pad('d', &short);
    browser.focus = LoopBrowserPane::Tracks;

    browser.handle_key(KeyCode::Char('c'));
    assert_eq!(browser.track_grid[0].len(), 2);
    assert_eq!(browser.track_grid[0][0].as_ref().unwrap().span_measures, 2);
    assert_eq!(browser.clip_at(0, 1).unwrap().0, 0);

    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('d'));
    assert_eq!(
        browser.track_grid[0][0].as_ref().unwrap().wav.relative,
        "short.wav"
    );
    assert!(browser.track_grid[0][1].is_none());

    browser.handle_key(KeyCode::Char('h'));
    browser.handle_key(KeyCode::Char('c'));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('c'));
    assert!(browser.track_grid[0].iter().all(Option::is_none));
}

#[test]
fn a_one_shot_span_rounds_up_to_a_power_of_two_without_an_upper_limit() {
    let mut browser = browser_with_spanning_wavs();
    browser.wav_analyses[1].1 = browser.wav_analyses[1].1.into_one_shot();
    let one_shot = browser.wav_analyses[1].0.clone();

    // 1 小節 = 2 秒として、鳴り終わるまでに要する小節数を 2 の冪へ切り上げる。
    for (duration_seconds, expected) in [
        (0.5, 1),
        (2.0, 1),
        (2.1, 2),
        (4.0, 2),
        (5.0, 4),
        (8.0, 4),
        (8.1, 8),
        (17.0, 16),
        (64.0, 32),
    ] {
        browser.wav_analyses[1].1.duration_seconds = duration_seconds;
        assert_eq!(
            browser.span_for_wav_at(&one_shot, 2.0),
            Some(expected),
            "{duration_seconds}s"
        );
    }

    // 壊れた解析値でグリッドの軸を暴走させない。
    browser.wav_analyses[1].1.duration_seconds = f64::NAN;
    assert_eq!(browser.span_for_wav_at(&one_shot, 2.0), Some(1));
    browser.wav_analyses[1].1.duration_seconds = 10.0;
    assert_eq!(browser.span_for_wav_at(&one_shot, 0.0), Some(1));
}
