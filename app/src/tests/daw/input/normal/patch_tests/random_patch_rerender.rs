use super::*;
use std::collections::BTreeSet;

#[test]
fn handle_normal_r_rerenders_playable_measures_without_rendering_measure_zero() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_rerender_logs");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let patch_path = tmp.join("Pad 1.fxp");
    std::fs::write(&patch_path, b"dummy").unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
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
        assert!(matches!(cache[1][2].state, CacheState::Rendering));
        let expected_generations = [cache[1][1].generation, cache[1][2].generation];
        drop(cache);

        let job1 = cache_rx
            .try_recv()
            .expect("highest-priority measure should be reserved");
        assert_eq!(
            (job1.measure, job1.generation),
            (1, expected_generations[0])
        );
        let job2 = cache_rx
            .try_recv()
            .expect("second measure should also be reserved when slots are available");
        assert_eq!(
            (job2.measure, job2.generation),
            (2, expected_generations[1])
        );
        assert!(
            cache_rx.try_recv().is_err(),
            "all pending measures should already be queued"
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
            logs.iter()
                .any(|line| line == "cache: rerender reserve track1 meas2 (meas2)"),
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
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 1;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
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
            "playback 中は cache rerender の予約を 1 小節に抑える"
        );

        let batches = app.track_rerender_batches.lock().unwrap();
        let batch = batches[1]
            .as_ref()
            .expect("remaining measure should stay pending in the batch");
        assert_eq!(batch.active_measures, BTreeSet::from([2]));
        assert!(batch.pending.contains_key(&1));
        drop(batches);

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
            !logs
                .iter()
                .any(|line| line == "cache: rerender reserve track1 meas1 (meas1)"),
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
fn handle_normal_r_restores_default_tempo_init_when_empty() {
    let tmp = std::env::temp_dir().join("cmrt_test_handle_normal_r_default_tempo_init");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();

    {
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 0;
        app.cursor_measure = 0;
        app.data[0][0] = "  ".to_string();
        app.data[1][1] = "cdef".to_string();
        app.play_measure_mmls.lock().unwrap()[0] = "stale".to_string();

        let result = app.handle_normal(crossterm::event::KeyCode::Char('r'));

        assert!(matches!(result, super::super::DawNormalAction::Continue));
        assert_eq!(app.data[0][0], crate::daw::DEFAULT_TRACK0_MML);
        assert_eq!(app.play_measure_mmls.lock().unwrap()[0], "t120cdef");
        assert_eq!(*app.play_measure_samples.lock().unwrap(), 176_400);

        let cache = app.cache.lock().unwrap();
        assert!(matches!(cache[0][0].state, CacheState::Empty));
        assert!(matches!(cache[1][1].state, CacheState::Rendering));
        drop(cache);

        let job = cache_rx
            .try_recv()
            .expect("tempo init restore should rerender affected playable measures");
        assert_eq!((job.track, job.measure), (1, 1));
        assert_eq!(job.mml, "t120cdef");
        assert!(
            cache_rx.try_recv().is_err(),
            "only non-empty playable measures should be queued"
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
        let _guard = crate::test_utils::set_local_dir_envs(&tmp);

        let (mut app, cache_rx) = build_test_app();
        app.cursor_track = 0;
        app.cursor_measure = 0;
        app.cfg = Arc::new(Config {
            patches_dirs: Some(vec![tmp.to_string_lossy().into_owned()]),
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
