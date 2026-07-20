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
fn track_pane_toggles_one_wav_per_cell_and_auto_extends_right_and_down() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    browser.handle_key(KeyCode::Tab);
    assert_eq!(browser.focus, LoopBrowserPane::Tracks);

    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::GridChanged { pad: 'c', audition, grid }
            if audition.ends_with("a.wav") && grid[0][0].as_ref().is_some_and(|clip| clip.path.ends_with("a.wav"))
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
        LoopBrowserAction::GridChanged { grid, .. } if grid[0][0].is_none()
    ));
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
    assert!(browser.track_grid[0][0].is_none());
    assert_eq!(
        browser.track_grid[0][1].as_ref().unwrap().wav.relative,
        "short.wav"
    );

    browser.handle_key(KeyCode::Char('h'));
    browser.handle_key(KeyCode::Char('c'));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('c'));
    assert!(browser.track_grid[0].iter().all(Option::is_none));
}
