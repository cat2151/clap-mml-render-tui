use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use super::{
    cache::try_insert_cache,
    render_queue::{TuiRenderCompletion, TuiRenderQueue, TuiRenderQueueStats, TuiRenderResponse},
    truncate_for_log, TuiApp,
};

const IMMEDIATE_DIRECTION_PREFETCH_STEPS: usize = 2;
const TOTAL_DIRECTION_PREFETCH_STEPS: usize = 4;

impl<'a> TuiApp<'a> {
    pub(super) fn sync_overlay_list_offset(
        state: &mut ratatui::widgets::ListState,
        cursor: usize,
        item_count: usize,
        page_size: usize,
    ) {
        if item_count == 0 {
            *state.offset_mut() = 0;
            return;
        }

        let visible_count = page_size.max(1).min(item_count);
        let margin = visible_count.div_ceil(3);
        let max_offset = item_count.saturating_sub(visible_count);
        let current_offset = state.offset().min(max_offset);
        let top_threshold = current_offset.saturating_add(margin);
        let bottom_anchor = visible_count.saturating_sub(margin + 1);
        let desired_offset = if cursor < top_threshold {
            cursor.saturating_sub(margin)
        } else if cursor > current_offset.saturating_add(bottom_anchor) {
            cursor.saturating_sub(bottom_anchor)
        } else {
            current_offset
        };
        *state.offset_mut() = desired_offset.min(max_offset);
    }

    fn filtered_prefetch_targets(&self, mmls: Vec<String>) -> Vec<String> {
        let cache = self.audio_cache.lock().unwrap();
        let mut targets = Vec::new();
        for mml in mmls.into_iter().map(|mml| mml.trim().to_string()) {
            if mml.is_empty() || cache.contains_key(&mml) || targets.contains(&mml) {
                continue;
            }
            targets.push(mml);
        }
        targets
    }

    #[cfg(test)]
    fn insert_prefetch_targets_for_tests(&self, targets: Vec<String>) {
        let mut cache = self.audio_cache.lock().unwrap();
        let mut cache_order = self.audio_cache_order.lock().unwrap();
        for mml in targets {
            try_insert_cache(&mut cache, &mut cache_order, mml, Vec::new(), false);
        }
    }

    fn queue_prefetch_targets(
        cache: &Arc<Mutex<HashMap<String, Vec<f32>>>>,
        render_queue: &TuiRenderQueue,
        targets: Vec<String>,
        prefetch_generation: u64,
    ) -> Vec<std::sync::mpsc::Receiver<TuiRenderResponse>> {
        targets
            .into_iter()
            .filter_map(|mml| {
                if cache.lock().unwrap().contains_key(&mml) {
                    return None;
                }
                match render_queue.submit_prefetch(mml.clone(), prefetch_generation) {
                    Ok(response_rx) => Some(response_rx),
                    Err(error) => {
                        Self::log_notepad_event(format!(
                            "cache prefetch queue error err=\"{}\" mml=\"{}\"",
                            truncate_for_log(&error.to_string(), 160),
                            truncate_for_log(&mml, 80)
                        ));
                        None
                    }
                }
            })
            .collect()
    }

    fn queue_idle_prefetch_targets(
        cache: &Arc<Mutex<HashMap<String, Vec<f32>>>>,
        render_queue: &TuiRenderQueue,
        idle_targets: &mut VecDeque<String>,
        prefetch_generation: u64,
        available_slots: usize,
    ) -> Vec<std::sync::mpsc::Receiver<TuiRenderResponse>> {
        let mut response_rxs = Vec::new();
        while response_rxs.len() < available_slots {
            let Some(next_idle) = idle_targets.pop_front() else {
                break;
            };
            response_rxs.extend(Self::queue_prefetch_targets(
                cache,
                render_queue,
                vec![next_idle],
                prefetch_generation,
            ));
        }
        response_rxs
    }

    fn consume_prefetch_response(
        cache: &Arc<Mutex<HashMap<String, Vec<f32>>>>,
        cache_order: &Arc<Mutex<VecDeque<String>>>,
        response_rx: std::sync::mpsc::Receiver<TuiRenderResponse>,
    ) {
        let Ok(response) = response_rx.recv() else {
            Self::log_notepad_event("cache prefetch render response dropped");
            return;
        };
        match response.completion {
            TuiRenderCompletion::Rendered { samples, .. } => {
                let mut cache = cache.lock().unwrap();
                let mut cache_order = cache_order.lock().unwrap();
                try_insert_cache(&mut cache, &mut cache_order, response.mml, samples, false);
                Self::log_notepad_event("cache prefetch render ok");
            }
            TuiRenderCompletion::RenderError(error) => {
                Self::log_notepad_event(format!(
                    "cache prefetch render error mml=\"{}\" err=\"{}\"",
                    truncate_for_log(&response.mml, 80),
                    truncate_for_log(&error, 160)
                ));
            }
            TuiRenderCompletion::SkippedStalePlayback => {}
        }
    }

    fn idle_prefetch_available_slots_for_stats(
        stats: TuiRenderQueueStats,
        active_render_count: usize,
        fill_to_worker_count: bool,
    ) -> usize {
        let outstanding_jobs = active_render_count.saturating_add(stats.pending_jobs);
        if fill_to_worker_count {
            stats.workers.saturating_sub(outstanding_jobs)
        } else if outstanding_jobs <= 1 {
            1
        } else {
            0
        }
    }

    fn idle_prefetch_available_slots(
        render_queue: &TuiRenderQueue,
        active_offline_render_count: &AtomicUsize,
        fill_to_worker_count: bool,
    ) -> usize {
        let stats = render_queue.stats();
        Self::idle_prefetch_available_slots_for_stats(
            stats,
            active_offline_render_count.load(Ordering::Relaxed),
            fill_to_worker_count,
        )
    }

    fn initial_idle_prefetch_available_slots_for_stats(
        stats: TuiRenderQueueStats,
        active_render_count: usize,
        has_immediate_responses: bool,
        fill_to_worker_count: bool,
    ) -> usize {
        if fill_to_worker_count || !has_immediate_responses {
            Self::idle_prefetch_available_slots_for_stats(
                stats,
                active_render_count,
                fill_to_worker_count,
            )
        } else {
            0
        }
    }

    fn initial_idle_prefetch_available_slots(
        render_queue: &TuiRenderQueue,
        active_offline_render_count: &AtomicUsize,
        has_immediate_responses: bool,
        fill_to_worker_count: bool,
    ) -> usize {
        let stats = render_queue.stats();
        Self::initial_idle_prefetch_available_slots_for_stats(
            stats,
            active_offline_render_count.load(Ordering::Relaxed),
            has_immediate_responses,
            fill_to_worker_count,
        )
    }

    pub(super) fn prefetch_audio_cache_with_idle_fill(
        &self,
        immediate_mmls: Vec<String>,
        idle_mmls: Vec<String>,
    ) {
        let immediate_targets = self.filtered_prefetch_targets(immediate_mmls);
        let idle_targets = self.filtered_prefetch_targets(idle_mmls);
        if immediate_targets.is_empty() && idle_targets.is_empty() {
            return;
        }
        let target_count = immediate_targets.len() + idle_targets.len();
        Self::log_notepad_event(format!("cache prefetch request count={target_count}"));

        #[cfg(test)]
        if self.entry_ptr == 0 {
            self.insert_prefetch_targets_for_tests(immediate_targets);
            if self.render_queue.stats().pending_jobs == 0 {
                self.insert_prefetch_targets_for_tests(idle_targets);
            }
            return;
        }

        let cache = Arc::clone(&self.audio_cache);
        let cache_order = Arc::clone(&self.audio_cache_order);
        let render_queue = self.render_queue.clone();
        let active_offline_render_count = Arc::clone(&self.active_offline_render_count);
        let prefetch_generation = render_queue.reserve_prefetch_generation();
        let fill_to_worker_count =
            self.cfg.offline_render_backend == crate::config::OfflineRenderBackend::RenderServer;
        let immediate_response_rxs = Self::queue_prefetch_targets(
            &cache,
            &render_queue,
            immediate_targets,
            prefetch_generation,
        );

        if immediate_response_rxs.is_empty() && idle_targets.is_empty() {
            return;
        }

        std::thread::spawn(move || {
            let mut idle_targets = VecDeque::from(idle_targets);
            let mut response_rxs = VecDeque::from(immediate_response_rxs);

            let available_slots = Self::initial_idle_prefetch_available_slots(
                &render_queue,
                &active_offline_render_count,
                !response_rxs.is_empty(),
                fill_to_worker_count,
            );
            if !idle_targets.is_empty() && available_slots > 0 {
                response_rxs.extend(Self::queue_idle_prefetch_targets(
                    &cache,
                    &render_queue,
                    &mut idle_targets,
                    prefetch_generation,
                    available_slots,
                ));
            }

            while let Some(response_rx) = response_rxs.pop_front() {
                Self::consume_prefetch_response(&cache, &cache_order, response_rx);
                let available_slots = Self::idle_prefetch_available_slots(
                    &render_queue,
                    &active_offline_render_count,
                    fill_to_worker_count,
                );
                if !idle_targets.is_empty() && available_slots > 0 {
                    response_rxs.extend(Self::queue_idle_prefetch_targets(
                        &cache,
                        &render_queue,
                        &mut idle_targets,
                        prefetch_generation,
                        available_slots,
                    ));
                }
            }
        });
    }

    pub(super) fn prefetch_navigation_audio_cache<F>(
        &self,
        current: usize,
        item_count: usize,
        page_size: usize,
        preferred_delta: Option<isize>,
        mml_for_index: F,
    ) where
        F: FnMut(usize) -> Option<String>,
    {
        let (immediate_indices, idle_indices) = match preferred_delta {
            Some(delta) if delta == 1 || delta == -1 => {
                let immediate_indices = crate::ui_utils::predicted_navigation_indices_in_direction(
                    current,
                    item_count,
                    delta,
                    IMMEDIATE_DIRECTION_PREFETCH_STEPS,
                );
                let idle_indices =
                    crate::ui_utils::predicted_navigation_indices_with_direction_bias(
                        current,
                        item_count,
                        page_size,
                        delta,
                        IMMEDIATE_DIRECTION_PREFETCH_STEPS,
                        TOTAL_DIRECTION_PREFETCH_STEPS,
                    )
                    .into_iter()
                    .filter(|index| !immediate_indices.contains(index))
                    .collect::<Vec<_>>();
                (immediate_indices, idle_indices)
            }
            Some(delta) => {
                let immediate_indices = crate::ui_utils::predicted_navigation_indices_in_direction(
                    current,
                    item_count,
                    delta,
                    IMMEDIATE_DIRECTION_PREFETCH_STEPS,
                );
                let idle_indices =
                    crate::ui_utils::predicted_navigation_indices(current, item_count, page_size)
                        .into_iter()
                        .filter(|index| !immediate_indices.contains(index))
                        .collect::<Vec<_>>();
                (immediate_indices, idle_indices)
            }
            None => (
                crate::ui_utils::predicted_navigation_indices(current, item_count, page_size),
                Vec::new(),
            ),
        };
        let mut mml_for_index = mml_for_index;
        let immediate_targets = immediate_indices
            .into_iter()
            .filter_map(&mut mml_for_index)
            .collect::<Vec<_>>();
        let idle_targets = idle_indices
            .into_iter()
            .filter_map(mml_for_index)
            .collect::<Vec<_>>();
        self.prefetch_audio_cache_with_idle_fill(immediate_targets, idle_targets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(workers: usize, pending_jobs: usize) -> TuiRenderQueueStats {
        TuiRenderQueueStats {
            workers,
            pending_jobs,
            pending_playback_jobs: 0,
        }
    }

    #[test]
    fn in_process_idle_prefetch_keeps_existing_relaxed_single_slot_policy() {
        assert_eq!(
            TuiApp::idle_prefetch_available_slots_for_stats(stats(2, 0), 0, false),
            1
        );
        assert_eq!(
            TuiApp::idle_prefetch_available_slots_for_stats(stats(2, 0), 1, false),
            1
        );
        assert_eq!(
            TuiApp::idle_prefetch_available_slots_for_stats(stats(2, 1), 1, false),
            0
        );
    }

    #[test]
    fn in_process_initial_idle_prefetch_waits_while_immediate_jobs_are_active() {
        assert_eq!(
            TuiApp::initial_idle_prefetch_available_slots_for_stats(stats(2, 0), 0, true, false),
            0
        );
        assert_eq!(
            TuiApp::initial_idle_prefetch_available_slots_for_stats(stats(2, 0), 0, false, false),
            1
        );
    }

    #[test]
    fn render_server_idle_prefetch_fills_to_worker_count() {
        assert_eq!(
            TuiApp::idle_prefetch_available_slots_for_stats(stats(4, 2), 0, true),
            2
        );
        assert_eq!(
            TuiApp::idle_prefetch_available_slots_for_stats(stats(4, 0), 2, true),
            2
        );
        assert_eq!(
            TuiApp::idle_prefetch_available_slots_for_stats(stats(4, 3), 1, true),
            0
        );
    }

    #[test]
    fn render_server_initial_idle_prefetch_fills_even_after_immediate_jobs() {
        assert_eq!(
            TuiApp::initial_idle_prefetch_available_slots_for_stats(stats(4, 2), 0, true, true),
            2
        );
    }
}
