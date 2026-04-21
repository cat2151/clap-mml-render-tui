use super::*;

impl<'a> TuiApp<'a> {
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
    ) -> Vec<std::sync::mpsc::Receiver<self::render_queue::TuiRenderResponse>> {
        let prefetch_generation = render_queue.reserve_prefetch_generation();
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

    fn consume_prefetch_response(
        cache: &Arc<Mutex<HashMap<String, Vec<f32>>>>,
        cache_order: &Arc<Mutex<VecDeque<String>>>,
        response_rx: std::sync::mpsc::Receiver<self::render_queue::TuiRenderResponse>,
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

    fn render_queue_is_relaxed(
        render_queue: &TuiRenderQueue,
        active_offline_render_count: &AtomicUsize,
    ) -> bool {
        let stats = render_queue.stats();
        active_offline_render_count.load(Ordering::Relaxed) + stats.pending_jobs <= 1
    }

    pub(in crate::tui) fn prefetch_audio_cache_with_idle_fill(
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
        let immediate_response_rxs =
            Self::queue_prefetch_targets(&cache, &render_queue, immediate_targets);

        if immediate_response_rxs.is_empty() && idle_targets.is_empty() {
            return;
        }

        std::thread::spawn(move || {
            let mut idle_targets = VecDeque::from(idle_targets);
            let mut response_rxs = VecDeque::from(immediate_response_rxs);

            if response_rxs.is_empty()
                && !idle_targets.is_empty()
                && Self::render_queue_is_relaxed(&render_queue, &active_offline_render_count)
            {
                if let Some(next_idle) = idle_targets.pop_front() {
                    response_rxs.extend(Self::queue_prefetch_targets(
                        &cache,
                        &render_queue,
                        vec![next_idle],
                    ));
                }
            }

            while let Some(response_rx) = response_rxs.pop_front() {
                Self::consume_prefetch_response(&cache, &cache_order, response_rx);
                if !idle_targets.is_empty()
                    && Self::render_queue_is_relaxed(&render_queue, &active_offline_render_count)
                {
                    if let Some(next_idle) = idle_targets.pop_front() {
                        response_rxs.extend(Self::queue_prefetch_targets(
                            &cache,
                            &render_queue,
                            vec![next_idle],
                        ));
                    }
                }
            }
        });
    }

    pub(in crate::tui) fn prefetch_navigation_audio_cache<F>(
        &self,
        current: usize,
        item_count: usize,
        page_size: usize,
        preferred_delta: Option<isize>,
        mml_for_index: F,
    ) where
        F: FnMut(usize) -> Option<String>,
    {
        let immediate_indices = match preferred_delta {
            Some(delta) => crate::ui_utils::predicted_navigation_indices_in_direction(
                current, item_count, delta, 2,
            ),
            None => crate::ui_utils::predicted_navigation_indices(current, item_count, page_size),
        };
        let idle_indices = preferred_delta
            .map(|_| crate::ui_utils::predicted_navigation_indices(current, item_count, page_size))
            .unwrap_or_default()
            .into_iter()
            .filter(|index| !immediate_indices.contains(index))
            .collect::<Vec<_>>();
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

    pub(in crate::tui) fn kick_play(&self, mml: String) {
        let cfg = Arc::clone(&self.cfg);
        let state = Arc::clone(&self.play_state);
        let playback_session = Arc::clone(&self.playback_session);
        let active_sink = Arc::clone(&self.active_sink);
        let cache = Arc::clone(&self.audio_cache);
        let cache_order = Arc::clone(&self.audio_cache_order);
        let render_queue = self.render_queue.clone();
        let session = self.begin_playback_session();
        let mml_log = truncate_for_log(&mml, 120);

        let cache_guard = cache.lock().unwrap();
        let cached_samples = resolve_cached_samples(Some(&cache_guard), &mml);
        if cached_samples.is_some() {
            let mut cache_order = cache_order.lock().unwrap();
            mark_cache_entry_recent(&cache_guard, &mut cache_order, &mml);
        }
        drop(cache_guard);

        if let Some(samples) = cached_samples {
            let msg = format!("(cached) | {}", mml);
            Self::log_notepad_event(format!(
                "play request session={session} cache=hit mml=\"{mml_log}\""
            ));
            self.set_play_state_if_current(session, PlayState::Playing(msg.clone()));

            std::thread::spawn(move || {
                Self::play_samples_for_session(
                    &state,
                    &playback_session,
                    &active_sink,
                    session,
                    cfg.sample_rate as u32,
                    samples,
                    msg,
                );
            });
        } else {
            Self::log_notepad_event(format!(
                "play request session={session} cache=miss mml=\"{mml_log}\""
            ));
            self.set_play_state_if_current(session, PlayState::Running(mml.clone()));

            let response_rx = match render_queue.submit_playback(
                mml.clone(),
                session,
                Arc::clone(&playback_session),
            ) {
                Ok(response_rx) => response_rx,
                Err(error) => {
                    Self::log_notepad_event(format!(
                        "play render queue error session={session} err=\"{}\"",
                        truncate_for_log(&error.to_string(), 160)
                    ));
                    self.set_play_state_if_current(
                        session,
                        PlayState::Err(format!("エラー: {}", error)),
                    );
                    return;
                }
            };

            std::thread::spawn(move || {
                let response = match response_rx.recv() {
                    Ok(response) => response,
                    Err(_) => {
                        Self::log_notepad_event(format!(
                            "play render response dropped session={session}"
                        ));
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err("エラー: render queue response dropped".to_string()),
                        );
                        return;
                    }
                };

                match response.completion {
                    TuiRenderCompletion::SkippedStalePlayback => {}
                    TuiRenderCompletion::RenderError(error) => {
                        Self::log_notepad_event(format!(
                            "play render error session={session} err=\"{}\"",
                            truncate_for_log(&error, 160)
                        ));
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err(format!("エラー: {}", error)),
                        );
                    }
                    TuiRenderCompletion::Rendered {
                        samples,
                        patch_name,
                    } => {
                        if !Self::playback_session_is_current(&playback_session, session) {
                            Self::log_notepad_event(format!(
                                "play render stale skip after-render session={session}"
                            ));
                            return;
                        }
                        {
                            let mut cache = cache.lock().unwrap();
                            let mut cache_order = cache_order.lock().unwrap();
                            try_insert_cache(
                                &mut cache,
                                &mut cache_order,
                                mml.clone(),
                                samples.clone(),
                                false,
                            );
                        }

                        let msg = format!("{} | {}", patch_name, mml);
                        Self::log_notepad_event(format!(
                            "play render ok session={session} patch=\"{}\"",
                            truncate_for_log(&patch_name, 120)
                        ));
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Playing(msg.clone()),
                        );
                        Self::play_samples_for_session(
                            &state,
                            &playback_session,
                            &active_sink,
                            session,
                            cfg.sample_rate as u32,
                            samples,
                            msg,
                        );
                    }
                }
            });
        }
    }

    pub(in crate::tui) fn active_parallel_render_count(&self) -> usize {
        self.active_offline_render_count.load(Ordering::Relaxed)
    }

    pub(in crate::tui) fn render_status_snapshot(&self) -> TuiRenderStatus {
        let queue_stats = self.render_queue.stats();
        TuiRenderStatus {
            active: self.active_parallel_render_count(),
            workers: queue_stats.workers,
            pending: queue_stats.pending_jobs,
            pending_playback: queue_stats.pending_playback_jobs,
        }
    }

    pub(in crate::tui) fn render_job_status_for_mml(
        &self,
        mml: &str,
    ) -> Option<TuiRenderJobStatus> {
        let mml = mml.trim();
        if mml.is_empty() {
            return None;
        }
        self.render_queue.job_status(mml)
    }
}
