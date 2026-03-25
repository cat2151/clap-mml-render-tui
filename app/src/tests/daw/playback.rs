use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tui_textarea::TextArea;

use crate::config::Config;

use super::{
    super::{CacheState, CellCache, DawApp, DawMode, DawPlayState},
    build_playback_measure_samples, current_play_measure_index, following_measure_index,
    format_playback_measure_advance_log, format_playback_measure_resolution_log,
};

/// stop_play のログ出力を検証するための最小構成の DawApp を作る。
fn build_test_app() -> DawApp {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        data: vec![vec![String::new(); measures + 1]; tracks],
        cursor_track: 0,
        cursor_measure: 0,
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
    }
}

#[test]
fn stop_play_logs_preview_stop_for_preview_state() {
    let app = build_test_app();
    *app.play_state.lock().unwrap() = DawPlayState::Preview;

    app.stop_play();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: stop")
    );
}

#[test]
fn stop_play_logs_play_stop_for_playing_state() {
    let app = build_test_app();
    *app.play_state.lock().unwrap() = DawPlayState::Playing;

    app.stop_play();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("play: stop")
    );
}

#[test]
fn current_play_measure_index_wraps_to_loop_start_when_measure_count_shrinks() {
    assert_eq!(current_play_measure_index(7, 4), 0);
    assert_eq!(current_play_measure_index(2, 4), 2);
}

#[test]
fn following_measure_index_wraps_after_last_measure() {
    assert_eq!(following_measure_index(1, 4), 2);
    assert_eq!(following_measure_index(3, 4), 0);
}

#[test]
fn format_playback_measure_resolution_log_shows_cursor_and_resolved_measure() {
    assert_eq!(
        format_playback_measure_resolution_log(7, 0, 4),
        "play: sync resolve cursor=meas8 -> current=meas1 (effective_count=4)"
    );
}

#[test]
fn format_playback_measure_advance_log_shows_current_and_next_measure() {
    assert_eq!(
        format_playback_measure_advance_log(1, 2, 4),
        "play: sync advance current=meas2 -> next=meas3 (effective_count=4)"
    );
}

#[test]
fn build_playback_measure_samples_returns_silence_for_empty_measure() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    let samples = build_playback_measure_samples(
        &cache,
        1,
        "",
        4,
        3,
        &log_lines,
        || -> Result<Vec<f32>, ()> { panic!("empty measure should not render") },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![0.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("meas2: empty -> silence")
    );
}

#[test]
fn build_playback_measure_samples_prefers_cache_hit() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5])),
        generation: 0,
        rendered_mml_hash: None,
    };

    let samples = build_playback_measure_samples(
        &cache,
        0,
        "c",
        4,
        3,
        &log_lines,
        || -> Result<Vec<f32>, ()> { panic!("cache hit should not render") },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5]);
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("meas1: cache hit track1/meas1")
    );
}

#[test]
fn build_playback_measure_samples_renders_and_normalizes_length() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1].state = CacheState::Pending;

    let samples = build_playback_measure_samples(
        &cache,
        0,
        "c",
        4,
        3,
        &log_lines,
        || -> Result<Vec<f32>, ()> { Ok(vec![1.0, 2.0]) },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![1.0, 2.0, 0.0, 0.0]);
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("meas1: render")
    );
}
