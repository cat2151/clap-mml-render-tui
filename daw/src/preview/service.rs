use std::sync::Arc;

use super::overlay_cache::OverlayPreviewCache;
use super::render::{
    overlay_preview_cache_key, render_mixed_preview_tracks, MixedPreviewRender,
    MixedPreviewRenderRequest, PreviewRenderProgress,
};
use crate::render_queue::{RenderPriority, RenderQueue};
use crate::MAX_CACHED_SAMPLES;

#[derive(Clone, Debug)]
pub(crate) struct OfflinePreviewRequest {
    pub(crate) measure_index: usize,
    pub(crate) measure_samples: usize,
    pub(crate) active_tracks: Vec<usize>,
    pub(crate) track_mmls: Vec<String>,
    pub(crate) track_gains: Vec<f32>,
    pub(crate) priority: RenderPriority,
}

impl OfflinePreviewRequest {
    pub(crate) fn current(
        measure_index: usize,
        measure_samples: usize,
        active_tracks: Vec<usize>,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
    ) -> Self {
        Self {
            measure_index,
            measure_samples,
            active_tracks,
            track_mmls,
            track_gains,
            priority: RenderPriority::High,
        }
    }

    pub(crate) fn prefetch(
        measure_index: usize,
        measure_samples: usize,
        active_tracks: Vec<usize>,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
    ) -> Self {
        Self {
            measure_index,
            measure_samples,
            active_tracks,
            track_mmls,
            track_gains,
            priority: RenderPriority::Low,
        }
    }
}

trait PreviewRenderer: Send + Sync {
    fn render_blocking(
        &self,
        request: &OfflinePreviewRequest,
        report_progress: &mut dyn FnMut(PreviewRenderProgress),
    ) -> Option<MixedPreviewRender>;

    #[cfg(test)]
    fn is_disabled(&self) -> bool {
        false
    }
}

struct QueuePreviewRenderer {
    queue: RenderQueue,
}

impl PreviewRenderer for QueuePreviewRenderer {
    fn render_blocking(
        &self,
        request: &OfflinePreviewRequest,
        report_progress: &mut dyn FnMut(PreviewRenderProgress),
    ) -> Option<MixedPreviewRender> {
        render_mixed_preview_tracks(
            &self.queue,
            MixedPreviewRenderRequest {
                priority: request.priority,
                measure_samples: request.measure_samples,
                active_tracks: &request.active_tracks,
                track_mmls: &request.track_mmls,
                track_gains: &request.track_gains,
                auto_trim: false,
            },
            report_progress,
        )
    }

    #[cfg(test)]
    fn is_disabled(&self) -> bool {
        self.queue.is_disabled()
    }
}

/// DAW overlay preview の cache と offline render 実行を束ねる cloneable handle。
#[derive(Clone)]
pub(crate) struct PreviewRenderService {
    cache: OverlayPreviewCache,
    renderer: Arc<dyn PreviewRenderer>,
}

impl PreviewRenderService {
    pub(crate) fn new(queue: RenderQueue, cache: OverlayPreviewCache) -> Self {
        Self {
            cache,
            renderer: Arc::new(QueuePreviewRenderer { queue }),
        }
    }

    #[cfg(test)]
    fn with_renderer(cache: OverlayPreviewCache, renderer: Arc<dyn PreviewRenderer>) -> Self {
        Self { cache, renderer }
    }

    pub(crate) fn cache_key(&self, request: &OfflinePreviewRequest) -> u64 {
        overlay_preview_cache_key(
            request.measure_index,
            &request.track_mmls,
            &request.track_gains,
        )
    }

    pub(crate) fn cached(&self, request: &OfflinePreviewRequest) -> Option<Arc<Vec<f32>>> {
        self.cache.get_cloned(self.cache_key(request))
    }

    pub(crate) fn store(&self, request: &OfflinePreviewRequest, samples: Arc<Vec<f32>>) {
        self.cache
            .insert(self.cache_key(request), samples.len(), samples);
    }

    /// 呼び出し thread を offline render 完了まで待たせる。
    /// UI thread から直接呼ばず、preview worker 内だけで使うこと。
    pub(crate) fn render_blocking<P>(
        &self,
        request: &OfflinePreviewRequest,
        mut report_progress: P,
    ) -> Option<MixedPreviewRender>
    where
        P: FnMut(PreviewRenderProgress),
    {
        self.renderer.render_blocking(request, &mut report_progress)
    }

    pub(crate) fn prefetch(&self, request: OfflinePreviewRequest) {
        if !self.should_prefetch(&request) {
            return;
        }

        #[cfg(test)]
        if self.renderer.is_disabled() {
            self.store(&request, Arc::new(Vec::new()));
            return;
        }

        let service = self.clone();
        std::thread::spawn(move || service.render_and_store_prefetch(request));
    }

    fn should_prefetch(&self, request: &OfflinePreviewRequest) -> bool {
        request.measure_samples <= MAX_CACHED_SAMPLES
            && !self.cache.contains_key(self.cache_key(request))
    }

    fn render_and_store_prefetch(&self, request: OfflinePreviewRequest) {
        let Some(render) = self.render_blocking(&request, |_| {}) else {
            return;
        };
        self.store(&request, Arc::new(render.samples));
    }

    #[cfg(test)]
    fn prefetch_blocking_for_test(&self, request: OfflinePreviewRequest) {
        if self.should_prefetch(&request) {
            self.render_and_store_prefetch(request);
        }
    }
}

#[cfg(test)]
mod tests;
