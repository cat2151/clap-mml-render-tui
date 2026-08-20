use std::sync::Arc;

use cmrt_core::NativeRenderProbeContext;

use super::{insert_overlay_preview_cache, overlay_preview_cache_key, render_mixed_preview_tracks};
use crate::render_queue::RenderPriority;
use crate::{DawApp, FIRST_PLAYABLE_TRACK, MAX_CACHED_SAMPLES};
use cmrt_history::daw_cache_mml_hash;

impl DawApp {
    pub(crate) fn prefetch_preview_navigation_cache<F>(
        &self,
        current: usize,
        item_count: usize,
        page_size: usize,
        preferred_delta: Option<isize>,
        mut preview_for_index: F,
    ) where
        F: FnMut(usize) -> Option<(usize, Vec<String>)>,
    {
        let track_gains = self.playback_track_gains();
        let predicted_indices = match preferred_delta {
            Some(delta) if delta == 1 || delta == -1 => {
                cmrt_tui_core::navigation::predicted_navigation_indices_with_direction_bias(
                    current, item_count, page_size, delta, 2, 4,
                )
            }
            _ => cmrt_tui_core::navigation::predicted_navigation_indices(
                current, item_count, page_size,
            ),
        };
        for index in predicted_indices {
            if let Some((measure_index, track_mmls)) = preview_for_index(index) {
                self.prefetch_preview_snapshot(measure_index, track_mmls, track_gains.clone());
            }
        }
    }

    pub(crate) fn prefetch_preview_snapshot(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
    ) {
        let active_tracks: Vec<usize> = (FIRST_PLAYABLE_TRACK..self.editor.tracks)
            .filter(|&track| {
                track_gains.get(track).copied().unwrap_or(1.0) > 0.0
                    && track_mmls
                        .get(track)
                        .map(|mml| !mml.trim().is_empty())
                        .unwrap_or(false)
            })
            .collect();
        if active_tracks.is_empty() {
            return;
        }

        let cache_key = overlay_preview_cache_key(measure_index, &track_mmls, &track_gains);
        if self
            .playback
            .overlay_preview_cache
            .lock()
            .unwrap()
            .contains_key(&cache_key)
        {
            return;
        }

        let measure_samples = self.measure_duration_samples();
        if measure_samples > MAX_CACHED_SAMPLES {
            return;
        }

        #[cfg(test)]
        if !self.plugin_entries.is_available() {
            insert_overlay_preview_cache(
                &mut self.playback.overlay_preview_cache.lock().unwrap(),
                cache_key,
                Arc::new(Vec::new()),
            );
            return;
        }
        let cfg = Arc::clone(&self.cfg);
        let render_queue = self.render_queue.clone();
        let overlay_preview_cache = Arc::clone(&self.playback.overlay_preview_cache);
        let active_track_count = active_tracks.len();
        std::thread::spawn(move || {
            let daw_cfg = (*cfg).clone();
            let offline_render_workers = daw_cfg.effective_offline_render_workers();
            let Some(samples) = render_mixed_preview_tracks(
                &render_queue,
                RenderPriority::Low,
                measure_samples,
                &active_tracks,
                &track_mmls,
                &track_gains,
                |track, mml| {
                    NativeRenderProbeContext::preview_prefetch(
                        track,
                        measure_index,
                        active_track_count,
                        daw_cache_mml_hash(mml),
                        offline_render_workers,
                    )
                },
            ) else {
                return;
            };
            insert_overlay_preview_cache(
                &mut overlay_preview_cache.lock().unwrap(),
                cache_key,
                Arc::new(samples),
            );
        });
    }
}
