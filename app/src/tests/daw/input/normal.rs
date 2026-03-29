use super::*;

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

#[test]
fn handle_normal_cursor_move_restarts_preview_on_new_target() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 1,
        measure_start: std::time::Instant::now(),
    });

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
    let logs = app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        logs.ends_with(&["preview: stop".to_string(), "preview: meas1".to_string()]),
        "logs: {:?}",
        logs
    );
}

#[test]
fn handle_normal_l_and_k_do_not_start_preview_while_playing() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 2;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 2);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert!(app.play_position.lock().unwrap().is_none());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_track, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert!(app.play_position.lock().unwrap().is_none());
}

#[test]
fn handle_normal_stops_preview_when_cursor_moves_to_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 0);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn handle_normal_stops_preview_when_cursor_moves_to_non_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_track, 0);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn normal_playback_shortcuts_map_correctly() {
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(NormalPlaybackShortcut::PreviewCurrentTrack)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        Some(NormalPlaybackShortcut::PreviewCurrentTrack)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        Some(NormalPlaybackShortcut::PreviewAllTracks)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT)),
        Some(NormalPlaybackShortcut::PlayFromCursor)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)),
        Some(NormalPlaybackShortcut::TogglePlay)
    );
    assert_eq!(
        normal_playback_shortcut(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE)),
        Some(NormalPlaybackShortcut::TogglePlay)
    );
}

#[test]
fn handle_normal_dd_yanks_current_measure_clears_it_and_records_patch_history() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "cdef".to_string();
    app.play_measure_mmls.lock().unwrap()[0] = "stale".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(app.normal_pending_delete);
    assert_eq!(app.data[1][1], "cdef");
    assert!(app.yank_buffer.is_none());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(!app.normal_pending_delete);
    assert_eq!(app.data[1][1], "");
    assert_eq!(app.yank_buffer.as_deref(), Some("cdef"));
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["cdef".to_string()])
    );
    assert!(app.patch_phrase_store_dirty);
    assert_eq!(app.play_measure_mmls.lock().unwrap()[0], "");
}

#[test]
fn handle_normal_p_overwrites_current_measure_from_yank_and_records_previous_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][0] = r#"{"Surge XT patch": "Pad 1.fxp"}"#.to_string();
    app.data[1][1] = "old".to_string();
    app.yank_buffer = Some("new".to_string());

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.data[1][1], "new");
    assert_eq!(app.yank_buffer.as_deref(), Some("new"));
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["old".to_string()])
    );
    assert!(app.patch_phrase_store_dirty);
}

#[test]
fn handle_normal_p_logs_when_yank_buffer_is_empty() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    app.data[1][1] = "old".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.data[1][1], "old");
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("ヤンクバッファが空です")
    );
}

#[test]
fn handle_normal_enter_stops_current_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn handle_normal_enter_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn handle_normal_enter_uses_test_preview_path_when_entry_ptr_is_unavailable() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|pos| pos.measure_index),
        Some(0)
    );
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: meas1")
    );
}

#[test]
fn handle_normal_shift_space_stops_current_preview() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Preview;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn handle_normal_shift_space_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn handle_normal_shift_enter_stops_current_play() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 1;
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    *app.play_position.lock().unwrap() = Some(PlayPosition {
        measure_index: 0,
        measure_start: std::time::Instant::now(),
    });

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert!(app.play_position.lock().unwrap().is_none());
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn preview_target_tracks_can_force_current_track_even_when_solo_mode_differs() {
    let target_tracks = preview_target_tracks(3, 2, false).expect("playable current track");

    assert_eq!(target_tracks, vec![2]);
}

#[test]
fn preview_target_tracks_can_temporarily_open_all_tracks() {
    let target_tracks = preview_target_tracks(3, 2, true).expect("all-track preview");

    assert_eq!(target_tracks, vec![1, 2]);
}

#[test]
fn preview_target_tracks_rejects_non_playable_current_track() {
    assert_eq!(preview_target_tracks(3, 0, false), None);
}

#[test]
fn play_from_cursor_uses_cursor_measure_index_for_start_position() {
    let start_measure_index =
        resolve_playback_start_measure_index(Some(1), NormalPlaybackShortcut::PlayFromCursor);

    assert_eq!(start_measure_index, Some(1));
}

#[test]
fn preview_shortcuts_keep_default_playback_start_position() {
    let start_measure_index =
        resolve_playback_start_measure_index(Some(1), NormalPlaybackShortcut::PreviewCurrentTrack);

    assert_eq!(start_measure_index, Some(0));
}
