use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tui_textarea::TextArea;

use crate::config::Config;

use super::super::{CacheState, CellCache, DawApp, DawMode, DawPlayState, PlayPosition};

fn build_test_app() -> (DawApp, std::sync::mpsc::Receiver<super::super::CacheJob>) {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    (
        DawApp {
            data: vec![vec![String::new(); measures + 1]; tracks],
            cursor_track: 1,
            cursor_measure: 1,
            mode: DawMode::Normal,
            textarea: TextArea::default(),
            cfg: Arc::new(Config {
                plugin_path: String::new(),
                input_midi: String::new(),
                output_midi: String::new(),
                output_wav: String::new(),
                sample_rate: 44_100.0,
                buffer_size: 512,
                patch_path: None,
                patches_dir: None,
                daw_tracks: tracks,
                daw_measures: measures,
            }),
            entry_ptr: 0,
            tracks,
            measures,
            cache: Arc::new(Mutex::new(vec![
                vec![CellCache::empty(); measures + 1];
                tracks
            ])),
            cache_tx,
            render_lock: Arc::new(Mutex::new(())),
            play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
            play_transition_lock: Arc::new(Mutex::new(())),
            play_position: Arc::new(Mutex::new(None)),
            play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
            play_measure_samples: Arc::new(Mutex::new(0)),
            log_lines: Arc::new(Mutex::new(VecDeque::new())),
            track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        },
        cache_rx,
    )
}

#[test]
fn commit_insert_skips_cache_refresh_when_text_is_unchanged() {
    let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_skips_cache_refresh");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, cache_rx) = build_test_app();
        app.data[1][1] = "cdef".to_string();
        {
            let mut cache = app.cache.lock().unwrap();
            cache[1][1].state = CacheState::Ready;
            cache[1][1].generation = 7;
        }

        app.start_insert();
        app.commit_insert();

        let cache = app.cache.lock().unwrap();
        assert_eq!(app.data[1][1], "cdef");
        assert!(matches!(cache[1][1].state, CacheState::Ready));
        assert_eq!(cache[1][1].generation, 7);
        assert!(
            cache_rx.try_recv().is_err(),
            "unchanged insert queued a cache job"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn commit_insert_triggers_cache_refresh_when_text_changes() {
    let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_refreshes_cache");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, cache_rx) = build_test_app();
        app.data[1][1] = "cdef".to_string();
        {
            let mut cache = app.cache.lock().unwrap();
            cache[1][1].state = CacheState::Ready;
            cache[1][1].generation = 7;
        }

        app.start_insert();
        app.textarea = TextArea::default();
        for ch in "gfed".chars() {
            app.textarea.insert_char(ch);
        }
        app.commit_insert();

        let cache = app.cache.lock().unwrap();
        assert_eq!(app.data[1][1], "gfed");
        assert!(matches!(cache[1][1].state, CacheState::Rendering));
        assert_eq!(cache[1][1].generation, 8);

        let job = cache_rx
            .try_recv()
            .expect("changed insert did not queue a cache job");
        assert_eq!(job.track, 1);
        assert_eq!(job.measure, 1);
        assert_eq!(job.generation, 8);
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn commit_insert_keeps_semicolon_text_in_same_measure() {
    let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_keeps_semicolon_text");
    std::fs::remove_dir_all(&tmp).ok();

    {
        let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

        let (mut app, cache_rx) = build_test_app();
        app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
        app.data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
        app.data[2][1] = "existing".to_string();

        app.start_insert();
        app.textarea = TextArea::default();
        for ch in "cde;gab".chars() {
            app.textarea.insert_char(ch);
        }
        app.commit_insert();

        assert_eq!(app.data[1][1], "cde;gab");
        assert_eq!(app.data[2][1], "existing");

        let job = cache_rx
            .try_recv()
            .expect("semicolon insert did not queue a cache job");
        assert_eq!(job.track, 1);
        assert_eq!(job.measure, 1);
        assert_eq!(
            job.mml.matches(r#"{"Surge XT patch": "piano"}"#).count(),
            2,
            "semicolon-separated phrases should each receive the track timbre: {}",
            job.mml
        );
        assert_eq!(
            job.mml.matches("t120").count(),
            2,
            "semicolon-separated phrases should each receive the track0/header content (t120): {}",
            job.mml
        );
        assert!(
            cache_rx.try_recv().is_err(),
            "unexpected extra cache job queued"
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
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
