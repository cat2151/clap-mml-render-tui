use super::*;

#[test]
fn handle_mixer_supports_track_navigation_and_escape() {
    let (mut app, _cache_rx) = build_test_app();
    app.mode = DawMode::Mixer;
    app.overlays.mixer.cursor_track = 2;

    app.handle_mixer(crossterm::event::KeyCode::Char('l'));
    assert_eq!(app.overlays.mixer.cursor_track, 3);

    app.handle_mixer(crossterm::event::KeyCode::Char('h'));
    assert_eq!(app.overlays.mixer.cursor_track, 2);

    app.handle_mixer(crossterm::event::KeyCode::Esc);
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_mixer_keeps_cursor_within_playable_track_range() {
    let (mut app, _cache_rx) = build_test_app();
    app.mode = DawMode::Mixer;
    app.overlays.mixer.cursor_track = 2;

    app.handle_mixer(crossterm::event::KeyCode::Left);
    assert_eq!(app.overlays.mixer.cursor_track, 2);

    app.overlays.mixer.cursor_track = app.editor.tracks - 1;
    app.handle_mixer(crossterm::event::KeyCode::Right);
    assert_eq!(app.overlays.mixer.cursor_track, app.editor.tracks - 1);
}

#[test]
fn handle_mixer_adjusts_volume_in_3db_steps() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_mixer_adjusts_volume");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
        let (mut app, _cache_rx) = build_test_app();
        app.mode = DawMode::Mixer;
        app.overlays.mixer.cursor_track = 2;

        app.handle_mixer(crossterm::event::KeyCode::Char('j'));
        app.handle_mixer(crossterm::event::KeyCode::Char('k'));
        app.handle_mixer(crossterm::event::KeyCode::Char('k'));

        assert_eq!(app.track_volume_db(2), 3);
        assert_eq!(app.playback_track_gains()[2], 10.0f32.powf(3.0 / 20.0));
    }

    std::fs::remove_dir_all(&tmp).ok();
}

/// 音量キー 1 回で live mix へ送る gain がちょうど 1 つだけ変わる。
///
/// 実際に送るのは演奏中だけ（`sync_live_track_gains`）なので、ここでは
/// **送る中身が 1 つだけ変わる**ところまでを実サーバー無しで固定する。
#[test]
fn handle_mixer_volume_key_changes_exactly_one_live_gain() {
    use crate::playback::live_gain::changed_live_track_gains;

    let tmp = std::env::temp_dir().join("cmrt_test_handle_mixer_live_gain");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
        let (mut app, _cache_rx) = build_test_app();
        app.mode = DawMode::Mixer;
        app.overlays.mixer.cursor_track = 2;
        let before = app.desired_live_track_gains();

        app.handle_mixer(crossterm::event::KeyCode::Char('j'));

        let changed = changed_live_track_gains(&before, &app.desired_live_track_gains());
        assert_eq!(changed.len(), 1, "1 打鍵で送るのは 1 instance 分だけ");
        assert_eq!(changed[0].row, 2);
        assert_eq!(changed[0].instance, 0);
        // 送る dB は mixer が見せている dB そのもの。
        assert_eq!(changed[0].gain_db, app.track_volume_db(2) as f32);
        assert_eq!(changed[0].gain_db, -3.0);
    }

    std::fs::remove_dir_all(&tmp).ok();
}
