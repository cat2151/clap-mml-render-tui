use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tui_textarea::TextArea;

use crate::config::Config;

use super::{
    super::{CacheState, CellCache, DawApp, DawMode, DawPlayState},
    cache_mixer::{
        build_playback_measure_samples, pad_playback_measure_samples, try_get_cached_samples,
    },
    measure_math::{
        current_play_measure_index, following_measure_index, format_playback_future_append_log,
        format_playback_measure_advance_log, format_playback_measure_resolution_log,
        future_chunk_append_deadline,
    },
    measure_mixer::{mix_measure_chunk, ActiveMeasureLayer},
    wait_until_or_stop,
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
fn future_chunk_append_deadline_uses_50ms_margin_before_next_measure() {
    let measure_start = Instant::now();
    let deadline = future_chunk_append_deadline(
        measure_start,
        Duration::from_millis(400),
        Duration::from_millis(50),
    );

    assert_eq!(
        deadline.duration_since(measure_start),
        Duration::from_millis(350)
    );
}

#[test]
fn future_chunk_append_deadline_clamps_to_measure_start_for_short_measures() {
    let measure_start = Instant::now();
    let deadline = future_chunk_append_deadline(
        measure_start,
        Duration::from_millis(30),
        Duration::from_millis(50),
    );

    assert_eq!(deadline, measure_start);
}

#[test]
fn format_playback_future_append_log_reports_append_lead_time() {
    let append_time = Instant::now();
    let measure_start = append_time + Duration::from_millis(48);

    assert_eq!(
        format_playback_future_append_log(2, append_time, measure_start, Duration::from_millis(50),),
        "play: queue meas3 append lead=48ms (target_margin=50ms)"
    );
}

#[test]
fn format_playback_future_append_log_reports_late_append() {
    let measure_start = Instant::now();
    let append_time = measure_start + Duration::from_millis(12);

    assert_eq!(
        format_playback_future_append_log(2, append_time, measure_start, Duration::from_millis(50),),
        "play: queue meas3 append late=12ms (target_margin=50ms)"
    );
}

#[test]
fn wait_until_or_stop_returns_false_when_playback_is_not_running() {
    let play_state = Arc::new(Mutex::new(DawPlayState::Idle));

    assert!(!wait_until_or_stop(
        &play_state,
        Instant::now() + Duration::from_millis(50)
    ));
}

#[test]
fn wait_until_or_stop_returns_true_when_deadline_is_already_reached() {
    let play_state = Arc::new(Mutex::new(DawPlayState::Playing));

    assert!(wait_until_or_stop(
        &play_state,
        Instant::now() - Duration::from_millis(1)
    ));
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

#[test]
fn build_playback_measure_samples_preserves_rendered_tail() {
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
        || -> Result<Vec<f32>, ()> { Ok(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]) },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn pad_playback_measure_samples_only_pads_short_buffers() {
    assert_eq!(
        pad_playback_measure_samples(vec![1.0, 2.0], 4),
        vec![1.0, 2.0, 0.0, 0.0]
    );
    assert_eq!(
        pad_playback_measure_samples(vec![1.0, 2.0, 3.0, 4.0, 5.0], 4),
        vec![1.0, 2.0, 3.0, 4.0, 5.0]
    );
}

#[test]
fn try_get_cached_samples_preserves_cached_tail_beyond_measure_length() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75])),
        generation: 0,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 3).unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75]);
    assert_eq!(samples.cached_tracks, vec![1]);
}

#[test]
fn mix_measure_chunk_overlaps_previous_tail_with_next_measure_start() {
    let mut active_layers = Vec::<ActiveMeasureLayer>::new();

    let first_chunk = mix_measure_chunk(&mut active_layers, vec![1.0; 6], 4);
    let second_chunk = mix_measure_chunk(&mut active_layers, vec![2.0; 4], 4);

    assert_eq!(first_chunk, vec![1.0, 1.0, 1.0, 1.0]);
    assert_eq!(second_chunk, vec![3.0, 3.0, 2.0, 2.0]);
    assert!(active_layers.is_empty());
}
