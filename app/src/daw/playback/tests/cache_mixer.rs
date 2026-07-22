use super::*;

#[test]
fn build_playback_measure_samples_returns_silence_for_empty_measure() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    let track_mmls = vec![String::new(); 3];
    let track_gains = playback_track_gains();
    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 1,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |_, _| -> Result<Vec<f32>, ()> { panic!("empty measure should not render") },
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
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };
    let track_mmls = playback_track_mmls(1, "c");
    let track_gains = playback_track_gains();

    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 0,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |_, _| -> Result<Vec<f32>, ()> { panic!("cache hit should not render") },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5]);
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("meas1: cache hit track1/meas1")
    );
}

#[test]
fn build_playback_measure_samples_uses_stale_cache_while_measure_is_pending() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Pending,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5])),
        rendered_measure_samples: Some(4),
        generation: 1,
        rendered_mml_hash: None,
    };
    let track_mmls = playback_track_mmls(1, "c");
    let track_gains = playback_track_gains();

    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 0,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |_, _| -> Result<Vec<f32>, ()> {
            panic!("stale cache should be reused while re-rendering")
        },
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
    let track_mmls = playback_track_mmls(1, "c");
    let track_gains = playback_track_gains();

    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 0,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |_, _| -> Result<Vec<f32>, ()> { Ok(vec![1.0, 2.0]) },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![1.0, 2.0, 0.0, 0.0]);
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("meas1: render")
    );
}

#[test]
fn build_playback_measure_samples_rerenders_when_stale_cache_measure_length_differs() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Pending,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75])),
        rendered_measure_samples: Some(6),
        generation: 1,
        rendered_mml_hash: None,
    };
    let track_mmls = playback_track_mmls(1, "c");
    let track_gains = playback_track_gains();

    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 0,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |_, _| -> Result<Vec<f32>, ()> { Ok(vec![1.0, 2.0, 3.0, 4.0]) },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![1.0, 2.0, 3.0, 4.0]);
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
    let track_mmls = playback_track_mmls(1, "c");
    let track_gains = playback_track_gains();

    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 0,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |_, _| -> Result<Vec<f32>, ()> { Ok(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]) },
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
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 3, &playback_track_gains()).unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75]);
    assert_eq!(samples.cached_tracks, vec![1]);
}

#[test]
fn try_get_cached_samples_uses_stale_samples_while_rendering() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Rendering,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5])),
        rendered_measure_samples: Some(4),
        generation: 1,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 3, &playback_track_gains()).unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5]);
    assert_eq!(samples.cached_tracks, vec![1]);
}

#[test]
fn try_get_cached_samples_rejects_stale_samples_when_measure_length_differs() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Rendering,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75])),
        rendered_measure_samples: Some(6),
        generation: 1,
        rendered_mml_hash: None,
    };

    assert!(try_get_cached_samples(&cache, 1, 4, 3, &playback_track_gains()).is_none());
}

#[test]
fn mark_cache_rendering_in_preserves_previous_samples_for_playback_fallback() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    let previous_samples = Arc::new(vec![0.25, -0.25, 0.5, -0.5]);
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::clone(&previous_samples)),
        rendered_measure_samples: Some(4),
        generation: 7,
        rendered_mml_hash: Some(42),
    };

    DawApp::mark_cache_rendering_in(&cache, 1, 1);

    let cache = cache.lock().unwrap();
    assert!(matches!(cache[1][1].state, CacheState::Rendering));
    assert_eq!(
        cache[1][1].samples.as_ref().map(|samples| samples.as_ref()),
        Some(previous_samples.as_ref())
    );
    assert_eq!(cache[1][1].rendered_measure_samples, Some(4));
    assert_eq!(cache[1][1].rendered_mml_hash, None);
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

#[test]
fn try_get_cached_samples_applies_track_gain_per_track() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    cache.lock().unwrap()[1][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![1.0, 1.0, 1.0, 1.0])),
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };
    cache.lock().unwrap()[2][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![1.0, 1.0, 1.0, 1.0])),
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 3, &[0.0, 1.0, 0.5]).unwrap();

    assert_eq!(samples.samples, vec![1.5, 1.5, 1.5, 1.5]);
}

#[test]
fn build_playback_measure_samples_renders_each_track_with_gain() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 3]));
    let track_mmls = vec![String::new(), "track1".to_string(), "track2".to_string()];
    let track_gains = vec![0.0, 1.0, 0.5];

    let samples = build_playback_measure_samples(
        &cache,
        PlaybackMeasureRequest {
            measure_index: 0,
            track_mmls: &track_mmls,
            measure_samples: 4,
            tracks: 3,
            track_gains: &track_gains,
        },
        &log_lines,
        |track, _| -> Result<Vec<f32>, ()> {
            Ok(if track == 1 {
                vec![1.0, 1.0, 1.0, 1.0]
            } else {
                vec![2.0, 2.0, 2.0, 2.0]
            })
        },
    )
    .unwrap();

    assert_eq!(samples.samples, vec![2.0, 2.0, 2.0, 2.0]);
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("meas1: render")
    );
}
