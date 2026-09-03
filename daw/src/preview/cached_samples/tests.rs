use std::sync::{Arc, Mutex};

use super::{pad_playback_measure_samples, try_get_cached_samples};
use crate::{CacheState, CellCache, DawApp};

/// 行 0 = Tempo / 行 1 = chord 行は鳴らさないので gain 0、行 2・3 が演奏 track。
fn preview_track_gains() -> Vec<f32> {
    vec![0.0, 0.0, 1.0, 1.0]
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
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 4]));
    cache.lock().unwrap()[2][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75])),
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 4, &preview_track_gains()).unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75]);
    assert_eq!(samples.cached_tracks, vec![2]);
}

#[test]
fn try_get_cached_samples_uses_stale_samples_while_rendering() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 4]));
    cache.lock().unwrap()[2][1] = CellCache {
        state: CacheState::Rendering,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5])),
        rendered_measure_samples: Some(4),
        generation: 1,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 4, &preview_track_gains()).unwrap();

    assert_eq!(samples.samples, vec![0.25, -0.25, 0.5, -0.5]);
    assert_eq!(samples.cached_tracks, vec![2]);
}

#[test]
fn try_get_cached_samples_rejects_stale_samples_when_measure_length_differs() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 4]));
    cache.lock().unwrap()[2][1] = CellCache {
        state: CacheState::Rendering,
        samples: Some(Arc::new(vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75])),
        rendered_measure_samples: Some(6),
        generation: 1,
        rendered_mml_hash: None,
    };

    assert!(try_get_cached_samples(&cache, 1, 4, 4, &preview_track_gains()).is_none());
}

#[test]
fn try_get_cached_samples_applies_track_gain_per_track() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 4]));
    cache.lock().unwrap()[2][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![1.0, 1.0, 1.0, 1.0])),
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };
    cache.lock().unwrap()[3][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::new(vec![1.0, 1.0, 1.0, 1.0])),
        rendered_measure_samples: Some(4),
        generation: 0,
        rendered_mml_hash: None,
    };

    let samples = try_get_cached_samples(&cache, 1, 4, 4, &[0.0, 0.0, 1.0, 0.5]).unwrap();

    assert_eq!(samples.samples, vec![1.5, 1.5, 1.5, 1.5]);
}

#[test]
fn mark_cache_rendering_in_preserves_previous_samples_for_preview_fallback() {
    let cache = Arc::new(Mutex::new(vec![vec![CellCache::empty(); 3]; 4]));
    let previous_samples = Arc::new(vec![0.25, -0.25, 0.5, -0.5]);
    cache.lock().unwrap()[2][1] = CellCache {
        state: CacheState::Ready,
        samples: Some(Arc::clone(&previous_samples)),
        rendered_measure_samples: Some(4),
        generation: 7,
        rendered_mml_hash: Some(42),
    };

    DawApp::mark_cache_rendering_in(&cache, 2, 1);

    let cache = cache.lock().unwrap();
    assert!(matches!(cache[2][1].state, CacheState::Rendering));
    assert_eq!(
        cache[2][1].samples.as_ref().map(|samples| samples.as_ref()),
        Some(previous_samples.as_ref())
    );
    assert_eq!(cache[2][1].rendered_measure_samples, Some(4));
    assert_eq!(cache[2][1].rendered_mml_hash, None);
}
