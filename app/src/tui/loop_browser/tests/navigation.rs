use super::*;

#[test]
fn track_navigation_reveals_start_continuation_and_repeat_wavs_in_the_tree() {
    let mut browser = browser_with_spanning_wavs();
    let long = browser.wav_analyses[0].0.clone();
    let short = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(long, 2)), None, None],
        vec![Some(LoopTrackClip::explicit(short, 1)), None, None],
    ];
    browser.normalize_track_grid();
    browser.focus = LoopBrowserPane::Tracks;

    browser.sync_tree_to_current_cell();
    assert_eq!(browser.visible[browser.cursor].name, "long.wav");

    browser.handle_key(KeyCode::Right);
    assert_eq!(browser.visible[browser.cursor].name, "long.wav");

    browser.handle_key(KeyCode::Down);
    assert_eq!(browser.visible[browser.cursor].name, "short.wav");
}

#[test]
fn tab_into_tracks_reveals_the_current_wav_without_moving_focus_back_to_tree() {
    let mut browser = browser_with_direct_wavs(1);
    let wav = browser.wav_analyses[0].0.clone();
    browser.track_grid[0][0] = Some(LoopTrackClip::explicit(wav, 1));

    browser.handle_key(KeyCode::Tab);

    assert_eq!(browser.focus, LoopBrowserPane::Tracks);
    assert_eq!(browser.visible[browser.cursor].name, "00.wav");
}

#[test]
fn random_replacement_keeps_every_tracks_following_repeat_populated() {
    let mut browser = browser_with_direct_wavs(5);
    let original = browser.wav_analyses[0].0.clone();
    let anchor = browser.wav_analyses[1].0.clone();
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(original.clone(), 1)), None],
        vec![Some(LoopTrackClip::explicit(original.clone(), 1)), None],
        vec![Some(LoopTrackClip::explicit(original, 1)), None],
        vec![None, Some(LoopTrackClip::explicit(anchor, 1))],
    ];
    let path = std::env::temp_dir()
        .join(format!(
            "cmrt-loop-random-multitrack-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("loop_browser")
        .join("random_decks.toml");
    browser.random_decks_path = Some(path.clone());
    browser.focus = LoopBrowserPane::Tracks;

    for track in 0..3 {
        browser.track_cursor = track;
        assert!(matches!(
            browser.handle_key(KeyCode::Char('r')),
            LoopBrowserAction::GridReplaced {
                start_measure: 0,
                ..
            }
        ));
    }

    let grid = browser.playback_grid();
    assert!(grid[0][1].is_some());
    assert!(grid[1][1].is_some());
    assert!(grid[2][1].is_some());
    assert!(browser
        .clip_at(0, 1)
        .is_some_and(|(_, clip)| clip.is_previous()));
    assert!(browser
        .clip_at(1, 1)
        .is_some_and(|(_, clip)| clip.is_previous()));
    assert!(browser
        .clip_at(2, 1)
        .is_some_and(|(_, clip)| clip.is_previous()));
    let _ = std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
}
