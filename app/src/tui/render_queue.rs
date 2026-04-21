use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
};

use anyhow::{anyhow, Result};
use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render_with_probe, CoreConfig, NativeRenderProbeContext};

use super::{truncate_for_log, ActiveRenderGuard};
use crate::{config::Config, history::daw_cache_mml_hash};

const MAX_TUI_RENDER_WORKERS: usize = 2;

#[path = "render_queue/worker.rs"]
mod worker;

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
    render_workers: usize,
    entry_ptr: usize,
    active_offline_render_count: Arc<std::sync::atomic::AtomicUsize>,
    state: Mutex<TuiRenderQueueState>,
    available: Condvar,
}

#[derive(Default)]
struct TuiRenderQueueState {
    jobs: HashMap<String, TuiRenderJob>,
}

struct TuiRenderJob {
    mml: String,
    state: TuiRenderJobState,
    waiters: Vec<TuiRenderWaiter>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TuiRenderJobState {
    Pending,
    Running,
}

struct TuiRenderWaiter {
    sequence: u64,
    kind: TuiRenderWaiterKind,
    response_tx: mpsc::Sender<TuiRenderResponse>,
}

enum TuiRenderWaiterKind {
    Playback {
        session: u64,
        playback_session: Arc<AtomicU64>,
    },
    Prefetch {
        generation: u64,
    },
}

struct StaleTuiRenderWaiter {
    mml: String,
    waiter: TuiRenderWaiter,
}

struct TuiRenderStart {
    stale_waiters: Vec<StaleTuiRenderWaiter>,
    work: Option<TuiRenderWork>,
}

struct TuiRenderWork {
    mml: String,
    caller: TuiRenderCaller,
}

#[derive(Clone, Copy)]
enum TuiRenderCaller {
    Playback { session: u64 },
    Prefetch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TuiRenderPriority {
    Prefetch(u64),
    Playback,
}

impl TuiRenderWaiter {
    fn priority(&self) -> TuiRenderPriority {
        match self.kind {
            TuiRenderWaiterKind::Playback { .. } => TuiRenderPriority::Playback,
            TuiRenderWaiterKind::Prefetch { generation } => TuiRenderPriority::Prefetch(generation),
        }
    }

    fn is_stale_playback(&self) -> bool {
        match &self.kind {
            TuiRenderWaiterKind::Playback {
                session,
                playback_session,
            } => playback_session.load(Ordering::Acquire) != *session,
            TuiRenderWaiterKind::Prefetch { .. } => false,
        }
    }
}

impl TuiRenderJob {
    fn new(mml: String, waiter: TuiRenderWaiter) -> Self {
        Self {
            mml,
            state: TuiRenderJobState::Pending,
            waiters: vec![waiter],
        }
    }

    fn effective_priority_sequence(&self) -> Option<(TuiRenderPriority, u64)> {
        let priority = self.waiters.iter().map(TuiRenderWaiter::priority).max()?;
        let sequence = self
            .waiters
            .iter()
            .filter(|waiter| waiter.priority() == priority)
            .map(|waiter| waiter.sequence)
            .min()?;
        Some((priority, sequence))
    }

    fn render_caller(&self) -> Option<TuiRenderCaller> {
        self.waiters
            .iter()
            .filter_map(|waiter| match &waiter.kind {
                TuiRenderWaiterKind::Playback { session, .. } => Some((waiter.sequence, *session)),
                TuiRenderWaiterKind::Prefetch { .. } => None,
            })
            .min_by_key(|(sequence, _)| *sequence)
            .map(|(_, session)| TuiRenderCaller::Playback { session })
            .or(Some(TuiRenderCaller::Prefetch))
    }
}

impl TuiRenderQueueState {
    fn stats(&self, workers: usize) -> TuiRenderQueueStats {
        let mut pending_jobs = 0;
        let mut pending_playback_jobs = 0;

        for job in self.jobs.values() {
            if job.state != TuiRenderJobState::Pending {
                continue;
            }

            pending_jobs += 1;
            if job
                .waiters
                .iter()
                .any(|waiter| matches!(&waiter.kind, TuiRenderWaiterKind::Playback { .. }))
            {
                pending_playback_jobs += 1;
            }
        }

        TuiRenderQueueStats {
            workers,
            pending_jobs,
            pending_playback_jobs,
        }
    }

    fn push_waiter(&mut self, mml: String, waiter: TuiRenderWaiter) {
        if let Some(job) = self.jobs.get_mut(&mml) {
            job.waiters.push(waiter);
            return;
        }
        self.jobs
            .insert(mml.clone(), TuiRenderJob::new(mml, waiter));
    }

    fn drain_stale_pending_playback_waiters(&mut self) -> Vec<StaleTuiRenderWaiter> {
        let keys = self
            .jobs
            .iter()
            .filter(|(_, job)| job.state == TuiRenderJobState::Pending)
            .map(|(mml, _)| mml.clone())
            .collect::<Vec<_>>();
        let mut stale_waiters = Vec::new();

        for mml in keys {
            let Some(job) = self.jobs.get_mut(&mml) else {
                continue;
            };
            let mut waiters = Vec::with_capacity(job.waiters.len());
            for waiter in job.waiters.drain(..) {
                if waiter.is_stale_playback() {
                    stale_waiters.push(StaleTuiRenderWaiter {
                        mml: mml.clone(),
                        waiter,
                    });
                } else {
                    waiters.push(waiter);
                }
            }
            job.waiters = waiters;
            if job.waiters.is_empty() {
                self.jobs.remove(&mml);
            }
        }

        stale_waiters
    }

    fn next_pending_key(&self) -> Option<String> {
        let mut best: Option<(&str, TuiRenderPriority, u64)> = None;
        for (mml, job) in &self.jobs {
            if job.state != TuiRenderJobState::Pending {
                continue;
            }
            let Some((priority, sequence)) = job.effective_priority_sequence() else {
                continue;
            };
            let should_replace = best
                .map(|(_, best_priority, best_sequence)| {
                    priority > best_priority
                        || (priority == best_priority && sequence < best_sequence)
                })
                .unwrap_or(true);
            if should_replace {
                best = Some((mml.as_str(), priority, sequence));
            }
        }
        best.map(|(mml, _, _)| mml.to_string())
    }

    fn job_status(&self, mml: &str) -> Option<TuiRenderJobStatus> {
        self.jobs.get(mml).map(|job| match job.state {
            TuiRenderJobState::Pending => TuiRenderJobStatus::Pending,
            TuiRenderJobState::Running => TuiRenderJobStatus::Running,
        })
    }
}

impl TuiRenderQueueInner {
    fn pop_next_render(&self) -> TuiRenderStart {
        let mut state = self.state.lock().unwrap();
        loop {
            let stale_waiters = state.drain_stale_pending_playback_waiters();
            if let Some(mml) = state.next_pending_key() {
                let job = state
                    .jobs
                    .get_mut(&mml)
                    .expect("selected render job must exist");
                job.state = TuiRenderJobState::Running;
                let caller = job
                    .render_caller()
                    .expect("selected render job must have a caller");
                return TuiRenderStart {
                    stale_waiters,
                    work: Some(TuiRenderWork {
                        mml: job.mml.clone(),
                        caller,
                    }),
                };
            }
            if !stale_waiters.is_empty() {
                return TuiRenderStart {
                    stale_waiters,
                    work: None,
                };
            }
            state = self.available.wait(state).unwrap();
        }
    }

    fn finish_render(&self, mml: &str) -> Vec<TuiRenderWaiter> {
        self.state
            .lock()
            .unwrap()
            .jobs
            .remove(mml)
            .map(|job| job.waiters)
            .unwrap_or_default()
    }
}

impl TuiRenderQueue {
    pub(super) fn new(
        cfg: Arc<Config>,
        entry_ptr: usize,
        active_offline_render_count: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Self {
        let configured_workers = cfg.offline_render_workers;
        let render_workers = render_worker_count(configured_workers);
        log_notepad_event(format!(
            "render queue workers={render_workers} configured_workers={configured_workers}"
        ));
        let inner = Arc::new(TuiRenderQueueInner {
            cfg,
            render_workers,
            entry_ptr,
            active_offline_render_count,
            state: Mutex::new(TuiRenderQueueState::default()),
            available: Condvar::new(),
        });
        for _ in 0..render_workers {
            let worker_inner = Arc::clone(&inner);
            std::thread::spawn(move || worker::render_worker(worker_inner));
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

#[cfg(test)]
#[path = "render_queue/tests.rs"]
mod tests;
