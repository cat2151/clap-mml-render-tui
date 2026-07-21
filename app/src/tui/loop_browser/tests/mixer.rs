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
        crate::mixer_overlay::MIXER_MIN_DB
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
