use super::*;

fn setup_tracks(browser: &mut LoopBrowser) {
    let wavs = browser
        .wav_analyses
        .iter()
        .map(|(wav, _)| wav.clone())
        .collect::<Vec<_>>();
    browser.track_grid = wavs
        .into_iter()
        .map(|wav| vec![Some(LoopTrackClip::explicit(wav, 1))])
        .collect();
    browser.track_volumes_db = vec![-3, 0, 6];
    browser.solo_tracks = vec![true, false, true];
    browser.focus = LoopBrowserPane::Tracks;
    browser.track_cursor = 1;
}

#[test]
fn alt_arrows_move_the_selected_track_with_mixer_and_solo_state() {
    let mut browser = browser_with_direct_wavs(3);
    setup_tracks(&mut browser);
    let path = std::env::temp_dir()
        .join(format!("cmrt-track-order-{}", std::process::id()))
        .join("track_grid.toml");
    browser.track_grid_path = Some(path.clone());

    let action = browser.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));

    assert_eq!(browser.track_cursor, 0);
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|clip| clip.wav.relative == "01.wav"));
    assert_eq!(browser.track_volumes_db, vec![0, -3, 6]);
    assert_eq!(browser.solo_tracks, vec![false, true, true]);
    assert!(matches!(
        action,
        LoopBrowserAction::TrackLayoutChanged {
            start_measure: 0,
            track_volumes_db,
            solo_tracks,
            ..
        } if track_volumes_db == vec![0, -3, 6]
            && solo_tracks == vec![false, true, true]
    ));

    let loaded = cmrt_loop_browser_domain::track_grid::load_from(&path).unwrap();
    assert_eq!(loaded.track_volumes_db, vec![0, -3, 6]);
    assert_eq!(loaded.grid, browser.track_grid);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn alt_arrows_do_nothing_at_track_list_boundaries() {
    let mut browser = browser_with_direct_wavs(3);
    setup_tracks(&mut browser);
    browser.track_cursor = 0;
    let original = browser.track_grid.clone();

    assert!(matches!(
        browser.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.track_grid, original);

    browser.track_cursor = 2;
    assert!(matches!(
        browser.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT)),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.track_grid, original);
}

#[test]
fn track_order_save_failure_rolls_back_every_track_state() {
    let mut browser = browser_with_direct_wavs(3);
    setup_tracks(&mut browser);
    let original_grid = browser.track_grid.clone();
    let original_volumes = browser.track_volumes_db.clone();
    let original_solos = browser.solo_tracks.clone();
    let blocked = std::env::temp_dir().join(format!(
        "cmrt-track-order-blocked-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&blocked).unwrap();
    browser.track_grid_path = Some(blocked.clone());

    assert!(matches!(
        browser.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.track_cursor, 1);
    assert_eq!(browser.track_grid, original_grid);
    assert_eq!(browser.track_volumes_db, original_volumes);
    assert_eq!(browser.solo_tracks, original_solos);
    assert!(browser
        .track_grid_error
        .as_deref()
        .is_some_and(|error| error.contains("track並び替えを保存できません")));
    let _ = std::fs::remove_dir_all(blocked);
}
