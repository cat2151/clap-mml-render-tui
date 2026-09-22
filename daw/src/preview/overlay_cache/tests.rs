use std::sync::Arc;

use super::OverlayPreviewCache;
use crate::{MAX_CACHED_SAMPLES, OVERLAY_PREVIEW_CACHE_MAX_ENTRIES};

#[test]
fn cache_miss_releases_lock_before_caller_fallback() {
    let cache = OverlayPreviewCache::default();

    assert!(cache.get_cloned(42).is_none());

    let _guard = cache
        .shared
        .entries
        .try_lock()
        .expect("cache miss must not retain the mutex guard");
}

#[test]
fn cache_hit_returns_owned_samples_and_releases_lock() {
    let cache = OverlayPreviewCache::default();
    let samples = Arc::new(vec![0.25, -0.25]);
    cache.insert(42, samples.len(), Arc::clone(&samples));

    let cached = cache.get_cloned(42).unwrap();

    assert!(Arc::ptr_eq(&cached, &samples));
    let _guard = cache
        .shared
        .entries
        .try_lock()
        .expect("cache hit must not retain the mutex guard");
}

#[test]
fn sample_limit_overflow_is_not_inserted() {
    let cache = OverlayPreviewCache::default();

    cache.insert(42, MAX_CACHED_SAMPLES + 1, Arc::new(Vec::new()));

    assert!(!cache.contains_key(42));
    assert_eq!(cache.entry_count(), 0);
}

#[test]
fn new_entry_at_entry_limit_evicts_all_existing_entries() {
    let cache = OverlayPreviewCache::default();
    for key in 0..OVERLAY_PREVIEW_CACHE_MAX_ENTRIES as u64 {
        cache.insert(key, 0, Arc::new(Vec::new()));
    }
    cache.insert(0, 0, Arc::new(vec![1.0]));

    assert_eq!(cache.entry_count(), OVERLAY_PREVIEW_CACHE_MAX_ENTRIES);
    assert!(cache.contains_key(1));

    let new_key = OVERLAY_PREVIEW_CACHE_MAX_ENTRIES as u64;
    cache.insert(new_key, 0, Arc::new(Vec::new()));

    assert_eq!(cache.entry_count(), 1);
    assert!(!cache.contains_key(0));
    assert!(cache.contains_key(new_key));
}

#[test]
fn entry_count_tracks_insert_and_clear() {
    let cache = OverlayPreviewCache::default();

    cache.insert(1, 0, Arc::new(Vec::new()));
    cache.insert(2, 0, Arc::new(Vec::new()));
    assert_eq!(cache.entry_count(), 2);

    cache.clear();
    assert_eq!(cache.entry_count(), 0);
}
