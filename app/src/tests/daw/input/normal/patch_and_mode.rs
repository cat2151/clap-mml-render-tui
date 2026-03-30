use super::*;

#[test]
fn handle_normal_g_sets_random_patch_and_generated_phrase_then_previews() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_g_generate");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let patch_path = tmp.join("Pad 1.fxp");
    std::fs::write(&patch_path, b"dummy").unwrap();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cfg = Arc::new(Config {
            patches_dir: Some(tmp.to_string_lossy().into_owned()),
            ..(*app.cfg).clone()
        });
        app.cursor_track = 1;
        app.cursor_measure = 1;
        app.data[1][0] = r#"{"Surge XT patch":"Old/Pad.fxp"}"#.to_string();
        app.data[1][1] = "old phrase".to_string();

        let result = app.handle_normal(crossterm::event::KeyCode::Char('g'));

        assert!(matches!(result, super::super::DawNormalAction::Continue));
        assert_eq!(app.data[1][0], r#"{"Surge XT patch":"Pad 1.fxp"}"#);
        assert!(
            app.data[1][1] == "c1" || app.data[1][1] == "cfg1",
            "generated phrase: {}",
            app.data[1][1]
        );
        assert_eq!(
            app.patch_phrase_store
                .patches
                .get("Old/Pad.fxp")
                .map(|state| state.history.clone()),
            Some(vec!["old phrase".to_string()])
        );
        assert!(matches!(
            *app.play_state.lock().unwrap(),
            DawPlayState::Preview
        ));
        assert_eq!(
            app.log_lines.lock().unwrap().back().map(String::as_str),
            Some("preview: meas1")
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_g_rejects_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 0;

    let result = app.handle_normal(crossterm::event::KeyCode::Char('g'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("generate は init 以外の小節でのみ使用できます")
    );
}

#[test]
fn skips_history_when_generate_is_noop() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch":"Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "c1".to_string();

    app.apply_generate_to_current_measure_with("Pad 1.fxp".to_string(), "c1", 0);

    assert_eq!(app.data[1][0], r#"{"Surge XT patch":"Pad 1.fxp"}"#);
    assert_eq!(app.data[1][1], "c1");
    assert!(app.patch_phrase_store.patches.is_empty());
    assert!(!app.patch_phrase_store_dirty);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
}

#[test]
fn handle_normal_r_rerenders_playable_measures_without_rendering_measure_zero() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_rerender_logs");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let patch_path = tmp.join("Pad 1.fxp");
    std::fs::write(&patch_path, b"dummy").unwrap();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dir: Some(tmp.to_string_lossy().into_owned()),
            ..(*app.cfg).clone()
        });
        app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
        app.data[1][1] = "cdef".to_string();
        app.data[1][2] = "gabc".to_string();
        app.track_volumes_db[1] = -6;
        // 共有 playback 状態を意図的に古い空データにしておき、
        // random patch 更新が hot reload 時に全共有 state を同期することを検証する。
        *app.play_measure_track_mmls.lock().unwrap() =
            vec![vec![String::new(); app.tracks]; app.measures];
        *app.play_track_gains.lock().unwrap() = vec![0.0; app.tracks];

        app.handle_normal(crossterm::event::KeyCode::Char('r'));

        assert_eq!(
            app.data[1][0], r#"{"Surge XT patch": "Pad 1.fxp"}"#,
            "random patch should update the timbre cell"
        );

        let cache = app.cache.lock().unwrap();
        assert!(matches!(cache[1][0].state, CacheState::Empty));
        assert!(matches!(cache[1][1].state, CacheState::Rendering));
        assert!(matches!(cache[1][2].state, CacheState::Pending));
        let expected_generations = [cache[1][1].generation, cache[1][2].generation];
        drop(cache);

        let job1 = cache_rx
            .try_recv()
            .expect("highest-priority measure should be reserved");
        assert_eq!(
            (job1.measure, job1.generation),
            (1, expected_generations[0])
        );
        assert!(
            cache_rx.try_recv().is_err(),
            "only one measure should be reserved at a time"
        );

        let logs = app
            .log_lines
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            logs.iter()
                .any(|line| line == "cache: rerender start track1 meas 1〜2 (random patch update)"),
            "logs: {:?}",
            logs
        );
        assert!(
            logs.iter()
                .any(|line| line == "cache: rerender reserve track1 meas1 (meas1 -> meas2)"),
            "logs: {:?}",
            logs
        );
        assert!(
            logs.iter().any(
                |line| line
                    == "play: hot reload random patch track1 display=none effective_count=None->Some(2) measure_samples=0->176400"
            ),
            "logs: {:?}",
            logs
        );
        let play_measure_track_mmls = app.play_measure_track_mmls.lock().unwrap().clone();
        assert!(
            play_measure_track_mmls[0][1].contains(r#"{"Surge XT patch": "Pad 1.fxp"}"#),
            "hot reload should refresh per-track playback MMLs: {:?}",
            play_measure_track_mmls
        );
        let play_track_gains = app.play_track_gains.lock().unwrap().clone();
        assert!(
            (play_track_gains[1] - track1_minus_6_db_gain()).abs() < f32::EPSILON,
            "hot reload should refresh playback gains: {:?}",
            play_track_gains
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_r_prioritizes_next_play_measure_when_playing() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_prioritizes_next_measure");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let patch_path = tmp.join("Pad 1.fxp");
    std::fs::write(&patch_path, b"dummy").unwrap();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dir: Some(tmp.to_string_lossy().into_owned()),
            ..(*app.cfg).clone()
        });
        app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
        app.data[1][1] = "cdef".to_string();
        app.data[1][2] = "gabc".to_string();
        *app.play_state.lock().unwrap() = DawPlayState::Playing;
        *app.play_position.lock().unwrap() = Some(PlayPosition {
            measure_index: 0,
            measure_start: std::time::Instant::now(),
        });
        *app.play_measure_mmls.lock().unwrap() = vec!["cdef".to_string(), "gabc".to_string()];

        app.handle_normal(crossterm::event::KeyCode::Char('r'));

        let reserved_job = cache_rx
            .try_recv()
            .expect("next playing measure should be reserved first");
        assert_eq!(reserved_job.measure, 2);
        assert!(
            cache_rx.try_recv().is_err(),
            "rerender should stay one-at-a-time even during playback"
        );

        let logs = app
            .log_lines
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            logs.iter()
                .any(|line| line == "cache: rerender reserve track1 meas2 (meas2 -> meas1)"),
            "logs: {:?}",
            logs
        );
        assert!(
            logs.iter().any(
                |line| line
                    == "play: hot reload random patch track1 display=meas1 effective_count=Some(2)->Some(2) measure_samples=0->176400"
            ),
            "logs: {:?}",
            logs
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_r_ignores_non_playable_track_and_keeps_header_unchanged() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_ignores_non_playable_track");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let patch_path = tmp.join("Pad 1.fxp");
    std::fs::write(&patch_path, b"dummy").unwrap();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 0;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dir: Some(tmp.to_string_lossy().into_owned()),
            ..(*app.cfg).clone()
        });
        app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();

        let result = app.handle_normal(crossterm::event::KeyCode::Char('r'));

        assert!(matches!(result, super::super::DawNormalAction::Continue));
        assert_eq!(app.data[0][0], r#"{"beat": "4/4"}t120"#);
        assert!(cache_rx.try_recv().is_err());
        assert_eq!(
            app.log_lines.lock().unwrap().back().map(String::as_str),
            Some("ランダム音色は演奏トラックでのみ使用できます")
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('?'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Help));
    assert!(matches!(app.help_origin, DawMode::Normal));
}

#[test]
fn handle_normal_n_returns_to_tui() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('n'));

    assert!(matches!(result, super::super::DawNormalAction::ReturnToTui));
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_normal_esc_has_no_effect() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 2;
    app.cursor_measure = 1;

    let result = app.handle_normal(crossterm::event::KeyCode::Esc);

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.cursor_track, 2);
    assert_eq!(app.cursor_measure, 1);
}

#[test]
fn handle_normal_a_cycles_ab_repeat_and_tracks_cursor_until_end_is_fixed() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_measure = 1;

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixStart {
            start_measure_index: 0,
            end_measure_index: 0,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Right);
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixStart {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Left);
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}

#[test]
fn handle_normal_a_can_turn_off_ab_repeat_from_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_measure = 1;
    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    app.handle_normal(crossterm::event::KeyCode::Char('a'));

    app.cursor_measure = 0;
    app.handle_normal(crossterm::event::KeyCode::Char('a'));

    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}

#[test]
fn handle_normal_s_enables_solo_for_current_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
    app.data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    app.data[1][1] = "cde".to_string();
    app.data[2][0] = r#"{"Surge XT patch": "brass"}"#.to_string();
    app.data[2][1] = "gab".to_string();

    app.handle_normal(crossterm::event::KeyCode::Char('s'));

    assert_eq!(app.solo_tracks, vec![false, true, false]);
    assert!(app.solo_mode_active());
    assert!(app.play_measure_mmls.lock().unwrap()[0].contains("cde"));
    assert!(!app.play_measure_mmls.lock().unwrap()[0].contains("gab"));
}

#[test]
fn handle_normal_s_toggles_tracks_and_turns_off_solo_mode_when_all_false() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;

    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, true, false]);

    app.cursor_track = 2;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, true, true]);

    app.cursor_track = 1;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, true]);
    assert!(app.solo_mode_active());

    app.cursor_track = 2;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, false]);
    assert!(!app.solo_mode_active());
}

#[test]
fn handle_normal_m_enters_mixer_mode_on_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 0;

    let result = app.handle_normal(crossterm::event::KeyCode::Char('m'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Mixer));
    assert_eq!(app.mixer_cursor_track, 1);
}

#[test]
fn handle_normal_h_and_j_preview_new_target_when_not_playing() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    app.data[1][1] = "cdef".to_string();
    app.data[2][1] = "gabc".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index),
        Some(0)
    );

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_track, 2);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: meas1")
    );
}
