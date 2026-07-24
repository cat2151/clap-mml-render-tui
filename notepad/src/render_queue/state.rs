use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
};

use super::{TuiRenderJobStatus, TuiRenderQueueInner, TuiRenderQueueStats, TuiRenderResponse};

#[derive(Default)]
pub(super) struct TuiRenderQueueState {
    pub(super) jobs: HashMap<String, TuiRenderJob>,
}

pub(super) struct TuiRenderJob {
    mml: String,
    state: TuiRenderJobState,
    pub(super) waiters: Vec<TuiRenderWaiter>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TuiRenderJobState {
    Pending,
    Running,
}

pub(super) struct TuiRenderWaiter {
    pub(super) sequence: u64,
    pub(super) kind: TuiRenderWaiterKind,
    pub(super) response_tx: mpsc::Sender<TuiRenderResponse>,
}

pub(super) enum TuiRenderWaiterKind {
    Playback {
        session: u64,
        playback_session: Arc<AtomicU64>,
    },
    Prefetch {
        generation: u64,
    },
}

pub(super) struct StaleTuiRenderWaiter {
    pub(super) mml: String,
    pub(super) waiter: TuiRenderWaiter,
}

pub(super) struct TuiRenderStart {
    pub(super) stale_waiters: Vec<StaleTuiRenderWaiter>,
    pub(super) work: Option<TuiRenderWork>,
}

pub(super) struct TuiRenderWork {
    pub(super) mml: String,
    pub(super) caller: TuiRenderCaller,
}

#[derive(Clone, Copy)]
pub(super) enum TuiRenderCaller {
    Playback { session: u64 },
    Prefetch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TuiRenderPriority {
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

    pub(super) fn effective_priority_sequence(&self) -> Option<(TuiRenderPriority, u64)> {
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
    pub(super) fn stats(&self, workers: usize) -> TuiRenderQueueStats {
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

    pub(super) fn push_waiter(&mut self, mml: String, waiter: TuiRenderWaiter) {
        if let Some(job) = self.jobs.get_mut(&mml) {
            job.waiters.push(waiter);
            return;
        }
        self.jobs
            .insert(mml.clone(), TuiRenderJob::new(mml, waiter));
    }

    pub(super) fn drain_stale_pending_playback_waiters(&mut self) -> Vec<StaleTuiRenderWaiter> {
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

    pub(super) fn next_pending_key(&self) -> Option<String> {
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

    pub(super) fn job_status(&self, mml: &str) -> Option<TuiRenderJobStatus> {
        self.jobs.get(mml).map(|job| match job.state {
            TuiRenderJobState::Pending => TuiRenderJobStatus::Pending,
            TuiRenderJobState::Running => TuiRenderJobStatus::Running,
        })
    }
}

impl TuiRenderQueueInner {
    pub(super) fn pop_next_render(&self) -> TuiRenderStart {
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

    pub(super) fn finish_render(&self, mml: &str) -> Vec<TuiRenderWaiter> {
        self.state
            .lock()
            .unwrap()
            .jobs
            .remove(mml)
            .map(|job| job.waiters)
            .unwrap_or_default()
    }
}
