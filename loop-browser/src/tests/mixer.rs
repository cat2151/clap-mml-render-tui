use super::*;

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
        cmrt_tui_core::mixer::MIXER_MIN_DB
    );
    assert!(matches!(
        browser.handle_key(KeyCode::Char('j')),
        LoopBrowserAction::Continue
    ));
}

#[test]
fn track_solo_key_supports_multiple_solos_and_restores_all_tracks() {
    let mut browser = browser();
    browser.focus = LoopBrowserPane::Tracks;

    assert!(matches!(
        browser.handle_key(KeyCode::Char('s')),
        LoopBrowserAction::TrackSoloChanged { solo_tracks }
            if solo_tracks == vec![true]
    ));
    assert!(browser.track_is_audible(0));

    browser.handle_key(KeyCode::Char('j'));
    assert_eq!(browser.solo_tracks, vec![true, false]);
    assert!(!browser.track_is_audible(1));
    assert!(matches!(
        browser.handle_key(KeyCode::Char('s')),
        LoopBrowserAction::TrackSoloChanged { solo_tracks }
            if solo_tracks == vec![true, true]
    ));

    browser.handle_key(KeyCode::Char('k'));
    browser.handle_key(KeyCode::Char('s'));
    assert_eq!(browser.solo_tracks, vec![false, true]);
    assert!(!browser.track_is_audible(0));
    assert!(browser.track_is_audible(1));

    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key(KeyCode::Char('s'));
    assert_eq!(browser.solo_tracks, vec![false, false]);
    assert!(browser.track_is_audible(0));
    assert!(browser.track_is_audible(1));
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
fn shift_m_replaces_existing_solos_with_two_random_nonempty_tracks() {
    let mut browser = browser_with_direct_wavs(3);
    let wavs = browser
        .wav_analyses
        .iter()
        .map(|(wav, _)| wav.clone())
        .collect::<Vec<_>>();
    browser.track_grid = vec![
        vec![Some(LoopTrackClip::explicit(wavs[0].clone(), 1))],
        vec![None],
        vec![Some(LoopTrackClip::explicit(wavs[1].clone(), 1))],
        vec![Some(LoopTrackClip::explicit(wavs[2].clone(), 1))],
    ];
    browser.solo_tracks = vec![true, true, true, true];
    browser.focus = LoopBrowserPane::Tracks;

    let action = browser.handle_key_event(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT));

    assert!(matches!(action, LoopBrowserAction::TrackSoloChanged { .. }));
    assert_eq!(browser.solo_tracks.iter().filter(|&&solo| solo).count(), 2);
    assert!(!browser.solo_tracks[1]);
}

#[test]
fn shift_m_solos_all_eligible_tracks_when_fewer_than_two_exist() {
    let mut browser = browser_with_direct_wavs(1);
    let wav = browser.wav_analyses[0].0.clone();
    browser.track_grid = vec![vec![None], vec![Some(LoopTrackClip::explicit(wav, 1))]];
    browser.solo_tracks = vec![true, false];
    browser.focus = LoopBrowserPane::Tracks;

    browser.handle_key_event(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT));
    assert_eq!(browser.solo_tracks, vec![false, true]);

    browser.track_grid[1][0] = None;
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT));
    assert_eq!(browser.solo_tracks, vec![false, false]);
}
