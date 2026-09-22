use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::{MAX_CACHED_SAMPLES, OVERLAY_PREVIEW_CACHE_MAX_ENTRIES};

#[derive(Clone, Default)]
pub(crate) struct OverlayPreviewCache {
    shared: Arc<OverlayPreviewCacheShared>,
}

#[derive(Default)]
struct OverlayPreviewCacheShared {
    entries: Mutex<HashMap<u64, Arc<Vec<f32>>>>,
    entry_count: AtomicUsize,
}

impl OverlayPreviewCache {
    pub(crate) fn get_cloned(&self, key: u64) -> Option<Arc<Vec<f32>>> {
        let lock_wait = crate::performance_log::SlowOperation::with_context(
            "preview-cache-lookup-lock",
            format!("cache_key={key}"),
        );
        let samples = self.shared.entries.lock().unwrap().get(&key).cloned();
        drop(lock_wait);
        samples
    }

    pub(crate) fn contains_key(&self, key: u64) -> bool {
        let lock_wait = crate::performance_log::SlowOperation::with_context(
            "preview-prefetch-cache-lock",
            format!("cache_key={key}"),
        );
        let contains_key = self.shared.entries.lock().unwrap().contains_key(&key);
        drop(lock_wait);
        contains_key
    }

    pub(crate) fn insert(&self, key: u64, sample_count: usize, samples: Arc<Vec<f32>>) {
        let mut entries = self.shared.entries.lock().unwrap();
        let _slow = crate::performance_log::SlowOperation::with_context(
            "overlay-preview-cache-insert-held",
            format!(
                "cache_key={key} entries_before={} sample_count={sample_count}",
                entries.len()
            ),
        );
        if sample_count > MAX_CACHED_SAMPLES {
            return;
        }
        if entries.len() >= OVERLAY_PREVIEW_CACHE_MAX_ENTRIES && !entries.contains_key(&key) {
            entries.clear();
        }
        entries.insert(key, samples);
        self.shared
            .entry_count
            .store(entries.len(), Ordering::Relaxed);
    }

    pub(crate) fn clear(&self) {
        let mut entries = self.shared.entries.lock().unwrap();
        entries.clear();
        self.shared
            .entry_count
            .store(entries.len(), Ordering::Relaxed);
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.shared.entry_count.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    pub(crate) fn with_entries_lock_held_for_test<F>(&self, callback: F)
    where
        F: FnOnce(),
    {
        let _guard = self.shared.entries.lock().unwrap();
        callback();
    }
}

#[cfg(test)]
mod tests;
