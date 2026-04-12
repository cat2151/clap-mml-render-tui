use super::*;

#[test]
fn handle_mixer_supports_track_navigation_and_escape() {
    let (mut app, _cache_rx) = build_test_app();
    app.mode = DawMode::Mixer;
    app.mixer_cursor_track = 1;

    app.handle_mixer(crossterm::event::KeyCode::Char('l'));
    assert_eq!(app.mixer_cursor_track, 2);

    app.handle_mixer(crossterm::event::KeyCode::Char('h'));
    assert_eq!(app.mixer_cursor_track, 1);

    app.handle_mixer(crossterm::event::KeyCode::Esc);
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_mixer_adjusts_volume_in_3db_steps() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_mixer_adjusts_volume");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);
        let (mut app, _cache_rx) = build_test_app();
        app.mode = DawMode::Mixer;
        app.mixer_cursor_track = 1;

        app.handle_mixer(crossterm::event::KeyCode::Char('j'));
        app.handle_mixer(crossterm::event::KeyCode::Char('k'));
        app.handle_mixer(crossterm::event::KeyCode::Char('k'));

        assert_eq!(app.track_volume_db(1), 3);
        assert_eq!(
            app.play_track_gains.lock().unwrap()[1],
            10.0f32.powf(3.0 / 20.0)
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}
