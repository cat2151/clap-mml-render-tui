use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
};

#[path = "render_queue/state.rs"]
mod state;

use anyhow::{anyhow, Result};
use cmrt_core::NativeRenderProbeContext;

use super::{truncate_for_log, ActiveRenderGuard};
use crate::{config::Config, history::daw_cache_mml_hash, offline_render::OfflineRenderer};

const MAX_TUI_RENDER_WORKERS: usize = 2;

#[derive(Clone)]
pub(super) struct TuiRenderQueue {
    inner: Option<Arc<TuiRenderQueueInner>>,
    disabled_stats: TuiRenderQueueStats,
    next_sequence: Arc<AtomicU64>,
    next_prefetch_generation: Arc<AtomicU64>,
    disabled_job_statuses: Arc<Mutex<HashMap<String, TuiRenderJobStatus>>>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct TuiRenderQueueStats {
    pub(super) workers: usize,
    pub(super) pending_jobs: usize,
    pub(super) pending_playback_jobs: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TuiRenderJobStatus {
    Pending,
    Running,
}

pub(super) struct TuiRenderResponse {
    pub(super) mml: String,
    pub(super) completion: TuiRenderCompletion,
}

#[derive(Clone)]
pub(super) enum TuiRenderCompletion {
    Rendered {
        samples: Vec<f32>,
        patch_name: String,
    },
    RenderError(String),
    SkippedStalePlayback,
}

struct TuiRenderQueueInner {
    cfg: Arc<Config>,
    renderer: OfflineRenderer,
    render_workers: usize,
    active_offline_render_count: Arc<std::sync::atomic::AtomicUsize>,
    state: Mutex<TuiRenderQueueState>,
    available: Condvar,
}

#[cfg(test)]
use state::TuiRenderPriority;
use state::{
    StaleTuiRenderWaiter, TuiRenderCaller, TuiRenderQueueState, TuiRenderWaiter,
    TuiRenderWaiterKind, TuiRenderWork,
};

impl TuiRenderQueue {
    pub(super) fn new(
        cfg: Arc<Config>,
        entry_ptr: usize,
        active_offline_render_count: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        let configured_workers = cfg.offline_render_workers;
        let render_workers = render_worker_count(configured_workers);
        let renderer = OfflineRenderer::new(Arc::clone(&cfg), entry_ptr);
        log_notepad_event(format!(
            "render queue workers={render_workers} configured_workers={configured_workers}"
        ));
        let inner = Arc::new(TuiRenderQueueInner {
            cfg,
            renderer,
            render_workers,
            active_offline_render_count,
            state: Mutex::new(TuiRenderQueueState::default()),
            available: Condvar::new(),
        });
        for _ in 0..render_workers {
            let worker_inner = Arc::clone(&inner);
            std::thread::spawn(move || render_worker(worker_inner));
        }
        Self {
            inner: Some(inner),
            disabled_stats: TuiRenderQueueStats::default(),
            next_sequence: Arc::new(AtomicU64::new(1)),
            next_prefetch_generation: Arc::new(AtomicU64::new(1)),
            disabled_job_statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    pub(super) fn disabled_for_tests(configured_workers: usize) -> Self {
        Self {
            inner: None,
            disabled_stats: TuiRenderQueueStats {
                workers: render_worker_count(configured_workers),
                pending_jobs: 0,
                pending_playback_jobs: 0,
            },
            next_sequence: Arc::new(AtomicU64::new(1)),
            next_prefetch_generation: Arc::new(AtomicU64::new(1)),
            disabled_job_statuses: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(super) fn stats(&self) -> TuiRenderQueueStats {
        let Some(inner) = &self.inner else {
            return self.disabled_stats;
        };
        inner.state.lock().unwrap().stats(inner.render_workers)
    }

    pub(super) fn job_status(&self, mml: &str) -> Option<TuiRenderJobStatus> {
        let Some(inner) = &self.inner else {
            return self.disabled_job_statuses.lock().unwrap().get(mml).copied();
        };
        inner.state.lock().unwrap().job_status(mml)
    }

    pub(super) fn reserve_prefetch_generation(&self) -> u64 {
        self.next_prefetch_generation
            .fetch_add(1, Ordering::Relaxed)
    }

    pub(super) fn submit_playback(
        &self,
        mml: String,
        session: u64,
        playback_session: Arc<AtomicU64>,
    ) -> Result<mpsc::Receiver<TuiRenderResponse>> {
        self.submit(
            mml,
            TuiRenderWaiterKind::Playback {
                session,
                playback_session,
            },
        )
    }

    pub(super) fn submit_prefetch(
        &self,
        mml: String,
        generation: u64,
    ) -> Result<mpsc::Receiver<TuiRenderResponse>> {
        self.submit(mml, TuiRenderWaiterKind::Prefetch { generation })
    }

    fn submit(
        &self,
        mml: String,
        kind: TuiRenderWaiterKind,
    ) -> Result<mpsc::Receiver<TuiRenderResponse>> {
        let Some(inner) = &self.inner else {
            return Err(anyhow!("TUI render queue is disabled"));
        };
        let (response_tx, response_rx) = mpsc::channel();
        let waiter = TuiRenderWaiter {
            sequence: self.next_sequence.fetch_add(1, Ordering::Relaxed),
            kind,
            response_tx,
        };
        inner.state.lock().unwrap().push_waiter(mml, waiter);
        inner.available.notify_one();
        Ok(response_rx)
    }

    #[cfg(test)]
    pub(super) fn set_test_job_status(
        &self,
        mml: impl Into<String>,
        status: Option<TuiRenderJobStatus>,
    ) {
        let mut statuses = self.disabled_job_statuses.lock().unwrap();
        let mml = mml.into();
        match status {
            Some(status) => {
                statuses.insert(mml, status);
            }
            None => {
                statuses.remove(&mml);
            }
        }
    }
}

fn render_worker_count(configured_workers: usize) -> usize {
    configured_workers.clamp(1, MAX_TUI_RENDER_WORKERS)
}

fn log_notepad_event(message: impl Into<String>) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("notepad: {}", message.into()));
    #[cfg(test)]
    let _ = message.into();
}

fn render_worker(inner: Arc<TuiRenderQueueInner>) {
    loop {
        let start = inner.pop_next_render();
        send_stale_skips(start.stale_waiters);
        let Some(work) = start.work else {
            continue;
        };
        let completion = render_work(&inner, &work);
        let waiters = inner.finish_render(&work.mml);
        send_completion(&work.mml, waiters, completion);
    }
}

fn render_work(inner: &TuiRenderQueueInner, work: &TuiRenderWork) -> TuiRenderCompletion {
    let _active_render_guard =
        ActiveRenderGuard::new(Arc::clone(&inner.active_offline_render_count));
    let active_render_count = inner.active_offline_render_count.load(Ordering::Relaxed);
    let probe_context = match work.caller {
        TuiRenderCaller::Playback { session } => {
            log_notepad_event(format!(
                "play render start session={session} active={} mml=\"{}\"",
                active_render_count,
                truncate_for_log(&work.mml, 120)
            ));
            NativeRenderProbeContext::tui_playback(
                session,
                active_render_count,
                daw_cache_mml_hash(&work.mml),
                inner.cfg.offline_render_workers,
            )
        }
        TuiRenderCaller::Prefetch => {
            log_notepad_event(format!(
                "cache prefetch render start active={} mml=\"{}\"",
                active_render_count,
                truncate_for_log(&work.mml, 80)
            ));
            NativeRenderProbeContext::tui_prefetch(
                active_render_count,
                daw_cache_mml_hash(&work.mml),
                inner.cfg.offline_render_workers,
            )
        }
    };

    match inner
        .renderer
        .render_phrase(&work.mml, Some(&probe_context))
    {
        Ok(rendered) => TuiRenderCompletion::Rendered {
            samples: rendered.samples,
            patch_name: rendered.patch_name,
        },
        Err(error) => TuiRenderCompletion::RenderError(error.to_string()),
    }
}

fn send_stale_skips(stale_waiters: Vec<StaleTuiRenderWaiter>) {
    for stale in stale_waiters {
        if let TuiRenderWaiterKind::Playback { session, .. } = stale.waiter.kind {
            log_notepad_event(format!(
                "play render stale skip before-render session={session}"
            ));
        }
        let _ = stale.waiter.response_tx.send(TuiRenderResponse {
            mml: stale.mml,
            completion: TuiRenderCompletion::SkippedStalePlayback,
        });
    }
}

fn send_completion(mml: &str, waiters: Vec<TuiRenderWaiter>, completion: TuiRenderCompletion) {
    for waiter in waiters {
        let _ = waiter.response_tx.send(TuiRenderResponse {
            mml: mml.to_string(),
            completion: completion.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefetch_waiter(sequence: u64) -> TuiRenderWaiter {
        prefetch_waiter_with_generation(sequence, 1)
    }

    fn prefetch_waiter_with_generation(sequence: u64, generation: u64) -> TuiRenderWaiter {
        let (response_tx, _response_rx) = mpsc::channel();
        TuiRenderWaiter {
            sequence,
            kind: TuiRenderWaiterKind::Prefetch { generation },
            response_tx,
        }
    }

    fn playback_waiter(
        sequence: u64,
        session: u64,
        playback_session: Arc<AtomicU64>,
    ) -> TuiRenderWaiter {
        let (response_tx, _response_rx) = mpsc::channel();
        TuiRenderWaiter {
            sequence,
            kind: TuiRenderWaiterKind::Playback {
                session,
                playback_session,
            },
            response_tx,
        }
    }

    #[test]
    fn queue_stats_count_pending_unique_jobs_and_pending_playback_jobs() {
        let playback_session = Arc::new(AtomicU64::new(1));
        let mut state = TuiRenderQueueState::default();

        state.push_waiter("prefetch".to_string(), prefetch_waiter(1));
        state.push_waiter(
            "playback".to_string(),
            playback_waiter(2, 1, Arc::clone(&playback_session)),
        );
        state.push_waiter(
            "playback".to_string(),
            playback_waiter(3, 1, Arc::clone(&playback_session)),
        );

        assert_eq!(
            state.stats(2),
            TuiRenderQueueStats {
                workers: 2,
                pending_jobs: 2,
                pending_playback_jobs: 1,
            }
        );
    }

    #[test]
    fn render_worker_count_caps_tui_workers_at_two() {
        assert_eq!(render_worker_count(0), 1);
        assert_eq!(render_worker_count(1), 1);
        assert_eq!(render_worker_count(2), 2);
        assert_eq!(render_worker_count(3), 2);
        assert_eq!(render_worker_count(4), 2);
    }

    #[test]
    fn pending_queue_prefers_playback_before_prefetch() {
        let playback_session = Arc::new(AtomicU64::new(1));
        let mut state = TuiRenderQueueState::default();

        state.push_waiter("prefetch".to_string(), prefetch_waiter(1));
        state.push_waiter(
            "playback".to_string(),
            playback_waiter(2, 1, Arc::clone(&playback_session)),
        );

        assert!(state.drain_stale_pending_playback_waiters().is_empty());
        assert_eq!(state.next_pending_key().as_deref(), Some("playback"));
    }

    #[test]
    fn pending_queue_deduplicates_same_mml_and_elevates_to_playback() {
        let playback_session = Arc::new(AtomicU64::new(1));
        let mut state = TuiRenderQueueState::default();

        state.push_waiter("same".to_string(), prefetch_waiter(1));
        state.push_waiter(
            "same".to_string(),
            playback_waiter(3, 1, Arc::clone(&playback_session)),
        );

        assert_eq!(state.jobs.len(), 1);
        let job = state.jobs.get("same").unwrap();
        assert_eq!(job.waiters.len(), 2);
        assert_eq!(
            job.effective_priority_sequence(),
            Some((TuiRenderPriority::Playback, 3))
        );
    }

    #[test]
    fn newer_prefetch_generation_runs_before_older_prefetch() {
        let mut state = TuiRenderQueueState::default();

        state.push_waiter("old".to_string(), prefetch_waiter_with_generation(1, 1));
        state.push_waiter("new".to_string(), prefetch_waiter_with_generation(2, 2));

        assert_eq!(state.next_pending_key().as_deref(), Some("new"));
    }

    #[test]
    fn stale_playback_waiters_are_dropped_before_render_selection() {
        let playback_session = Arc::new(AtomicU64::new(2));
        let mut state = TuiRenderQueueState::default();

        state.push_waiter(
            "old".to_string(),
            playback_waiter(1, 1, Arc::clone(&playback_session)),
        );
        state.push_waiter("prefetch".to_string(), prefetch_waiter(2));

        let stale = state.drain_stale_pending_playback_waiters();

        assert_eq!(stale.len(), 1);
        assert!(!state.jobs.contains_key("old"));
        assert_eq!(state.next_pending_key().as_deref(), Some("prefetch"));
    }
}
