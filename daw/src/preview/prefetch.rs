use super::service::OfflinePreviewRequest;
use crate::{DawApp, FIRST_PLAYABLE_TRACK};

impl DawApp {
    pub(crate) fn prefetch_preview_navigation_cache<F>(
        &self,
        current: usize,
        item_count: usize,
        page_size: usize,
        preferred_delta: Option<isize>,
        preview_for_index: F,
    ) where
        F: FnMut(usize) -> Option<(usize, Vec<String>)>,
    {
        self.prefetch_preview_navigation_cache_with_gains(
            self.playback_track_gains(),
            current,
            item_count,
            page_size,
            preferred_delta,
            preview_for_index,
        );
    }

    /// `track_gains` は overlay preview cache のキーに入るので、実際に再生するときと
    /// 同じものを渡すこと（違うと prefetch した結果が使われない）。
    pub(crate) fn prefetch_preview_navigation_cache_with_gains<F>(
        &self,
        track_gains: Vec<f32>,
        current: usize,
        item_count: usize,
        page_size: usize,
        preferred_delta: Option<isize>,
        mut preview_for_index: F,
    ) where
        F: FnMut(usize) -> Option<(usize, Vec<String>)>,
    {
        let _slow = crate::performance_log::SlowOperation::with_context(
            "preview-navigation-prefetch",
            format!(
                "current={current} item_count={item_count} page_size={page_size} preferred_delta={preferred_delta:?}"
            ),
        );
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

        let measure_samples = self.measure_duration_samples();
        self.render
            .preview_service()
            .prefetch(OfflinePreviewRequest::prefetch(
                measure_index,
                measure_samples,
                active_tracks,
                track_mmls,
                track_gains,
            ));
    }
}
