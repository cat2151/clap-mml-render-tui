use super::*;

#[test]
fn persisted_analysis_drives_browser_without_opening_the_wav() {
    let analysis = LoopWavAnalysis {
        duration_seconds: 5.25,
        bpm: 137.5,
        beats: 9,
        meter_numerator: 3,
        meter_denominator: 4,
        measures: 3,
        source: LoopAnalysisSource::Acid,
    };
    let mut browser = LoopBrowser::from_index(
        LoopIndex {
            version: 2,
            roots: vec![LoopRootIndex {
                path: "/path/that/does/not/exist".to_string(),
                wav_files: vec![LoopWavIndex {
                    relative: "cached.wav".to_string(),
                    analysis,
                }],
            }],
        },
        &crate::config::default_loop_categories(),
        LoopBrowserMetadata::default(),
        None,
        true,
        None,
    );
    let wav = browser.wav_analyses[0].0.clone();
    browser.metadata.toggle_pad('c', &wav);
    browser.focus = LoopBrowserPane::Tracks;

    browser.handle_key(KeyCode::Char('c'));
    let grid = browser.playback_grid();
    let clip = grid[0][0].as_ref().unwrap();

    assert_eq!(browser.track_grid[0][0].as_ref().unwrap().span_measures, 3);
    assert_eq!(clip.bpm, 137.5);
    assert_eq!(clip.meter_numerator, 3);
    assert_eq!(clip.meter_denominator, 4);
}

#[test]
fn target_bpm_tracks_all_placed_wavs_and_returns_to_120_after_removal() {
    let mut browser = browser_with_direct_wavs(2);
    browser.wav_analyses[0].1.bpm = 160.0;
    browser.wav_analyses[1].1.bpm = 120.0;
    let fast = browser.wav_analyses[0].0.clone();
    let normal = browser.wav_analyses[1].0.clone();
    browser.track_grid[0] = vec![
        Some(LoopTrackClip {
            wav: fast,
            span_measures: 1,
        }),
        Some(LoopTrackClip {
            wav: normal,
            span_measures: 1,
        }),
    ];

    assert_eq!(browser.target_bpm().bpm, 128.0);

    browser.track_grid[0][0] = None;
    assert_eq!(browser.target_bpm().bpm, 120.0);
}

#[test]
fn stretch_limit_display_check_uses_the_selected_target_bpm() {
    let mut browser = browser_with_direct_wavs(2);
    browser.wav_analyses[0].1.bpm = 100.0;
    browser.wav_analyses[1].1.bpm = 200.0;
    let compatible = LoopTrackClip {
        wav: browser.wav_analyses[0].0.clone(),
        span_measures: 1,
    };
    let rejected = LoopTrackClip {
        wav: browser.wav_analyses[1].0.clone(),
        span_measures: 1,
    };
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
        LoopBrowserAction::GridRefresh(grid)
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
        LoopBrowserAction::GridRefresh(grid) if grid[0][0].is_none()
    ));
}

#[test]
fn every_pad_key_places_and_removes_without_audition() {
    for pad in PAD_KEYS {
        let mut browser = browser_with_direct_wavs(1);
        let wav = browser.wav_analyses[0].0.clone();
        browser.metadata.toggle_pad(pad, &wav);
        browser.focus = LoopBrowserPane::Tracks;

        assert!(matches!(
            browser.handle_key(KeyCode::Char(pad)),
            LoopBrowserAction::GridRefresh(grid) if grid[0][0].is_some()
        ));
        assert!(matches!(
            browser.handle_key(KeyCode::Char(pad)),
            LoopBrowserAction::GridRefresh(grid) if grid[0][0].is_none()
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
        browser.track_grid[0][0] = Some(LoopTrackClip {
            wav: original,
            span_measures: 2,
        });
        browser.metadata.toggle_pad(pad, &replacement);
        browser.focus = LoopBrowserPane::Tracks;
        browser.measure_cursor = 1;

        assert!(matches!(
            browser.handle_key(KeyCode::Char(pad)),
            LoopBrowserAction::GridReplaced {
                start_measure: 0,
                grid,
            } if grid[0][0]
                .as_ref()
                .is_some_and(|clip| clip.path.ends_with("short.wav"))
                && grid[0].len() == 1
        ));
    }
}

#[test]
fn playback_grid_uses_content_end_and_repeats_one_measure_clips_across_gaps() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    let short = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![
            Some(LoopTrackClip {
                wav: short.clone(),
                span_measures: 1,
            }),
            None,
            None,
            None,
        ],
        vec![
            Some(LoopTrackClip {
                wav: long,
                span_measures: 4,
            }),
            None,
            None,
            None,
        ],
        vec![
            Some(LoopTrackClip {
                wav: short.clone(),
                span_measures: 1,
            }),
            None,
            Some(LoopTrackClip {
                wav: short,
                span_measures: 1,
            }),
            None,
        ],
    ];

    let grid = browser.playback_grid();

    assert!(grid[0].iter().all(Option::is_some));
    assert!(grid[1][0].is_some());
    assert!(grid[1][1..].iter().all(Option::is_none));
    assert!(grid[2].iter().all(Option::is_some));
    assert!(grid[2][3].is_some());
    assert!(browser.previous_measure_repeat_clip(0, 1).is_some());
    assert!(browser.previous_measure_repeat_clip(1, 2).is_none());
    assert!(browser.previous_measure_repeat_clip(2, 1).is_some());
    assert!(browser.previous_measure_repeat_clip(2, 3).is_some());
}

#[test]
fn allocated_empty_columns_do_not_extend_playback_or_normal_display() {
    let mut browser = browser_with_direct_wavs(1);
    let wav = browser.wav_analyses[0].0.clone();
    browser.track_grid[0] = vec![
        Some(LoopTrackClip {
            wav,
            span_measures: 1,
        }),
        None,
        None,
        None,
        None,
        None,
    ];

    assert_eq!(browser.playback_grid()[0].len(), 1);
    assert_eq!(browser.displayed_measure_count(), 1);
    assert!(browser.previous_measure_repeat_clip(0, 1).is_none());

    browser.measure_cursor = 5;
    assert_eq!(browser.displayed_measure_count(), 6);
    assert!(browser.previous_measure_repeat_clip(0, 5).is_none());
}

#[test]
fn track_pad_write_failures_do_not_audition_or_change_the_grid() {
    let mut browser = browser_with_direct_wavs(2);
    let original = browser.wav_analyses[0].0.clone();
    let replacement = browser.wav_analyses[1].0.clone();
    browser.track_grid[0][0] = Some(LoopTrackClip {
        wav: original.clone(),
        span_measures: 1,
    });
    browser.metadata.toggle_pad('c', &replacement);
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
fn track_mixer_opens_on_selected_track_and_adjusts_in_three_db_steps() {
    let mut browser = browser();
    browser.focus = LoopBrowserPane::Tracks;
    browser.handle_key(KeyCode::Char('j'));
    assert_eq!(browser.track_cursor, 1);

    assert!(matches!(
        browser.handle_key(KeyCode::Char('m')),
        LoopBrowserAction::Continue
    ));
    assert!(browser.mixer_overlay_open);
    assert_eq!(browser.mixer_cursor_track, 1);

    assert!(matches!(
        browser.handle_key(KeyCode::Char('k')),
        LoopBrowserAction::TrackVolumeChanged {
            track: 1,
            volume_db: 3
        }
    ));
    assert_eq!(browser.track_volume_db(1), 3);

    browser.handle_key(KeyCode::Char('h'));
    assert_eq!(browser.mixer_cursor_track, 0);
    browser.handle_key(KeyCode::Left);
    assert_eq!(browser.mixer_cursor_track, 0);
    browser.handle_key(KeyCode::Esc);
    assert!(!browser.mixer_overlay_open);
    assert_eq!(browser.track_cursor, 1);
}

#[test]
fn track_mixer_clamps_volume_to_shared_bounds() {
    let mut browser = browser();
    browser.focus = LoopBrowserPane::Tracks;
    browser.handle_key(KeyCode::Char('m'));

    for _ in 0..20 {
        browser.handle_key(KeyCode::Char('j'));
    }
    assert_eq!(
        browser.track_volume_db(0),
        crate::mixer_overlay::MIXER_MIN_DB
    );
    assert!(matches!(
        browser.handle_key(KeyCode::Char('j')),
        LoopBrowserAction::Continue
    ));
}

#[test]
fn track_mixer_rolls_back_when_persistence_fails() {
    let mut browser = browser();
    browser.focus = LoopBrowserPane::Tracks;
    let dir = std::env::temp_dir().join(format!(
        "cmrt-loop-mixer-blocked-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    browser.track_grid_path = Some(dir.clone());

    browser.handle_key(KeyCode::Char('m'));
    assert!(matches!(
        browser.handle_key(KeyCode::Char('k')),
        LoopBrowserAction::Continue
    ));

    assert_eq!(browser.track_volume_db(0), 0);
    assert!(browser
        .track_grid_error
        .as_deref()
        .is_some_and(|error| error.contains("mix levelを保存できません")));
    let _ = std::fs::remove_dir_all(dir);
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
    browser.metadata.toggle_pad('c', &long);
    browser.metadata.toggle_pad('d', &short);
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
