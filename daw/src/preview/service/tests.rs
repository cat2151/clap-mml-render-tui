use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::{OfflinePreviewRequest, PreviewRenderService, PreviewRenderer};
use crate::preview::overlay_cache::OverlayPreviewCache;
use crate::preview::render::{
    overlay_preview_cache_key, MixedPreviewRender, PreviewRenderProgress,
};
use crate::render_queue::RenderPriority;
use crate::{FIRST_PLAYABLE_TRACK, MAX_CACHED_SAMPLES};

#[derive(Default)]
struct FakeRenderer {
    calls: Mutex<Vec<OfflinePreviewRequest>>,
    results: Mutex<VecDeque<Option<MixedPreviewRender>>>,
}

impl FakeRenderer {
    fn with_results(results: impl IntoIterator<Item = Option<MixedPreviewRender>>) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            results: Mutex::new(results.into_iter().collect()),
        }
    }
}

impl PreviewRenderer for FakeRenderer {
    fn render_blocking(
        &self,
        request: &OfflinePreviewRequest,
        _report_progress: &mut dyn FnMut(PreviewRenderProgress),
    ) -> Option<MixedPreviewRender> {
        self.calls.lock().unwrap().push(request.clone());
        self.results.lock().unwrap().pop_front().flatten()
    }
}

fn request(measure_index: usize, priority: RenderPriority) -> OfflinePreviewRequest {
    let mut track_mmls = vec![String::new(); FIRST_PLAYABLE_TRACK + 1];
    track_mmls[FIRST_PLAYABLE_TRACK] = format!("o4c{measure_index}");
    let track_gains = vec![1.0; track_mmls.len()];
    let active_tracks = vec![FIRST_PLAYABLE_TRACK];
    match priority {
        RenderPriority::High => OfflinePreviewRequest::current(
            measure_index,
            16,
            active_tracks,
            track_mmls,
            track_gains,
        ),
        RenderPriority::Low => OfflinePreviewRequest::prefetch(
            measure_index,
            16,
            active_tracks,
            track_mmls,
            track_gains,
        ),
        RenderPriority::Normal => panic!("preview request must not use Normal priority"),
    }
}

fn service_with_fake(fake: Arc<FakeRenderer>) -> PreviewRenderService {
    PreviewRenderService::with_renderer(OverlayPreviewCache::default(), fake)
}

fn rendered(samples: Vec<f32>) -> Option<MixedPreviewRender> {
    Some(MixedPreviewRender {
        samples,
        auto_trim_volumes_db: None,
    })
}

#[test]
fn request_cache_key_matches_existing_helper() {
    let service = service_with_fake(Arc::new(FakeRenderer::default()));
    let request = request(3, RenderPriority::High);

    assert_eq!(
        service.cache_key(&request),
        overlay_preview_cache_key(
            request.measure_index,
            &request.track_mmls,
            &request.track_gains,
        )
    );
}

#[test]
fn cache_hit_does_not_submit_prefetch_render() {
    let fake = Arc::new(FakeRenderer::default());
    let service = service_with_fake(Arc::clone(&fake));
    let request = request(0, RenderPriority::Low);
    service.store(&request, Arc::new(vec![0.25]));

    service.prefetch_blocking_for_test(request);

    assert!(fake.calls.lock().unwrap().is_empty());
}

#[test]
fn prefetch_stores_only_successful_render() {
    let fake = Arc::new(FakeRenderer::with_results([
        rendered(vec![0.25, -0.25]),
        None,
    ]));
    let service = service_with_fake(fake);
    let successful = request(0, RenderPriority::Low);
    let failed = request(1, RenderPriority::Low);

    service.prefetch_blocking_for_test(successful.clone());
    service.prefetch_blocking_for_test(failed.clone());

    assert_eq!(
        service.cached(&successful).unwrap().as_slice(),
        &[0.25, -0.25]
    );
    assert!(service.cached(&failed).is_none());
}

#[test]
fn oversized_render_result_is_not_cached() {
    let fake = Arc::new(FakeRenderer::with_results([rendered(vec![
        0.0;
        MAX_CACHED_SAMPLES
            + 1
    ])]));
    let service = service_with_fake(fake);
    let request = request(0, RenderPriority::Low);

    service.prefetch_blocking_for_test(request.clone());

    assert!(service.cached(&request).is_none());
}

#[test]
fn current_and_prefetch_requests_keep_high_and_low_priorities() {
    let fake = Arc::new(FakeRenderer::with_results([None, None]));
    let service = service_with_fake(Arc::clone(&fake));
    let current = request(0, RenderPriority::High);
    let prefetch = request(1, RenderPriority::Low);

    assert!(service.render_blocking(&current, |_| {}).is_none());
    service.prefetch_blocking_for_test(prefetch);

    let priorities = fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .map(|request| request.priority)
        .collect::<Vec<_>>();
    assert_eq!(priorities, vec![RenderPriority::High, RenderPriority::Low]);
}
