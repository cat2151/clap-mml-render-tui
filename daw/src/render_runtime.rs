use std::sync::Arc;
use std::time::Instant;

use crate::preview::overlay_cache::OverlayPreviewCache;
use crate::preview::service::PreviewRenderService;
use crate::render_queue::{RenderQueue, RenderQueueStatusLog};

/// DAW の offline render queue、preview cache、status snapshot を所有する runtime。
pub(crate) struct DawRenderRuntime {
    queue: RenderQueue,
    overlay_preview_cache: OverlayPreviewCache,
    preview_service: PreviewRenderService,
    status_log: RenderQueueStatusLog,
}

impl DawRenderRuntime {
    pub(crate) fn new(cfg: Arc<cmrt_runtime::Config>, workers: usize) -> Self {
        let queue = RenderQueue::new(cfg, workers);
        let overlay_preview_cache = OverlayPreviewCache::default();
        Self {
            preview_service: PreviewRenderService::new(
                queue.clone(),
                overlay_preview_cache.clone(),
            ),
            queue,
            overlay_preview_cache,
            status_log: RenderQueueStatusLog::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn disabled_for_tests() -> Self {
        let queue = RenderQueue::disabled_for_tests();
        let overlay_preview_cache = OverlayPreviewCache::default();
        Self {
            preview_service: PreviewRenderService::new(
                queue.clone(),
                overlay_preview_cache.clone(),
            ),
            queue,
            overlay_preview_cache,
            status_log: RenderQueueStatusLog::default(),
        }
    }

    /// cell cache worker を preview と同じ queue へ接続するための handle。
    pub(crate) fn queue_handle(&self) -> RenderQueue {
        self.queue.clone()
    }

    pub(crate) fn preview_service(&self) -> PreviewRenderService {
        self.preview_service.clone()
    }

    #[cfg(test)]
    pub(crate) fn preview_cache(&self) -> OverlayPreviewCache {
        self.overlay_preview_cache.clone()
    }

    pub(crate) fn clear_preview_cache(&self) {
        self.overlay_preview_cache.clear();
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.queue.is_disabled()
    }

    /// UI thread が queue/cache の内部同期へ触れずに取得する status snapshot。
    pub(crate) fn status_line_if_due(&mut self, now: Instant) -> Option<String> {
        self.status_log.line_if_due(
            now,
            self.queue.snapshot(),
            self.overlay_preview_cache.entry_count(),
        )
    }
}

#[cfg(test)]
mod tests;
