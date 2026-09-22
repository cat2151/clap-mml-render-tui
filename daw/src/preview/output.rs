use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{DawPlayState, PlayPosition};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PreviewSessionToken(u64);

#[derive(Clone)]
pub(crate) struct PreviewOutputHandle {
    shared: Arc<PreviewOutputShared>,
}

struct PreviewOutputShared {
    transition_lock: Arc<Mutex<()>>,
    enqueue_lock: Mutex<()>,
    play_state: Arc<Mutex<DawPlayState>>,
    session: AtomicU64,
    sink: Mutex<Option<Arc<rodio::Player>>>,
    position: Arc<Mutex<Option<PlayPosition>>>,
}

impl PreviewOutputHandle {
    pub(crate) fn new(
        transition_lock: Arc<Mutex<()>>,
        play_state: Arc<Mutex<DawPlayState>>,
        position: Arc<Mutex<Option<PlayPosition>>>,
    ) -> Self {
        Self {
            shared: Arc::new(PreviewOutputShared {
                transition_lock,
                enqueue_lock: Mutex::new(()),
                play_state,
                session: AtomicU64::new(0),
                sink: Mutex::new(None),
                position,
            }),
        }
    }

    /// 前の preview output を止め、新しい session を current にする。
    pub(crate) fn start_session(&self) -> PreviewSessionToken {
        self.start_session_with(|| {})
    }

    pub(crate) fn start_session_with<F>(&self, stop_previous_output: F) -> PreviewSessionToken
    where
        F: FnOnce(),
    {
        let transition_lock_wait =
            crate::performance_log::SlowOperation::new("preview-transition-lock");
        let transition_guard = self.shared.transition_lock.lock().unwrap();
        drop(transition_lock_wait);
        let enqueue_guard = self.shared.enqueue_lock.lock().unwrap();

        let sink_lock_wait = crate::performance_log::SlowOperation::new("preview-sink-lock");
        let previous_sink = self.shared.sink.lock().unwrap().take();
        drop(sink_lock_wait);
        if let Some(sink) = previous_sink {
            let _slow = crate::performance_log::SlowOperation::new("preview-stop-previous-sink");
            sink.stop();
        }
        stop_previous_output();
        let position_lock_wait =
            crate::performance_log::SlowOperation::new("preview-position-lock");
        *self.shared.position.lock().unwrap() = None;
        drop(position_lock_wait);
        let token = PreviewSessionToken(self.shared.session.fetch_add(1, Ordering::AcqRel) + 1);
        let state_lock_wait = crate::performance_log::SlowOperation::new("preview-state-lock");
        *self.shared.play_state.lock().unwrap() = DawPlayState::Preview;
        drop(state_lock_wait);

        drop(enqueue_guard);
        drop(transition_guard);
        token
    }

    pub(crate) fn is_current(&self, token: PreviewSessionToken) -> bool {
        let state_lock_wait = crate::performance_log::SlowOperation::new("preview-state-lock");
        let current = *self.shared.play_state.lock().unwrap() == DawPlayState::Preview
            && self.shared.session.load(Ordering::Acquire) == token.0;
        drop(state_lock_wait);
        current
    }

    /// 通常再生への遷移を preview の finish/start と直列化する。
    pub(crate) fn mark_playing(&self) {
        let _transition_guard = self.shared.transition_lock.lock().unwrap();
        let _enqueue_guard = self.shared.enqueue_lock.lock().unwrap();
        let state_lock_wait = crate::performance_log::SlowOperation::new("preview-state-lock");
        *self.shared.play_state.lock().unwrap() = DawPlayState::Playing;
        drop(state_lock_wait);
    }

    /// current session だけに再生位置と sink を関連付けて samples を enqueue する。
    ///
    /// callback の実行中は transition/state lock を保持しない。別 thread の stop/start は
    /// `enqueue_lock` で callback 完了まで直列化されるため、stale session の enqueue と
    /// cleanup が前後して音声が復活することはない。
    pub(crate) fn enqueue_if_current<F>(
        &self,
        token: PreviewSessionToken,
        measure_index: usize,
        measure_duration: Duration,
        sink: Option<Arc<rodio::Player>>,
        enqueue_audio: F,
    ) -> bool
    where
        F: FnOnce(),
    {
        let transition_guard = self.shared.transition_lock.lock().unwrap();
        let enqueue_guard = self.shared.enqueue_lock.lock().unwrap();
        if !self.is_current(token) {
            return false;
        }
        let position_lock_wait =
            crate::performance_log::SlowOperation::new("preview-position-lock");
        *self.shared.position.lock().unwrap() = Some(PlayPosition {
            measure_index,
            measure_start: Instant::now(),
            measure_duration,
        });
        drop(position_lock_wait);
        if let Some(sink) = sink {
            *self.shared.sink.lock().unwrap() = Some(sink);
        }
        drop(transition_guard);

        enqueue_audio();
        drop(enqueue_guard);
        true
    }

    pub(crate) fn wait_until_end(&self, token: PreviewSessionToken, duration: Duration) {
        let deadline = Instant::now() + duration;
        loop {
            if !self.is_current(token) {
                return;
            }
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            std::thread::sleep((deadline - now).min(Duration::from_millis(10)));
        }
    }

    /// error/cancel/finish の通知を current session にだけ反映する。
    pub(crate) fn finish_session(&self, token: PreviewSessionToken) -> bool {
        self.finish_session_with(token, || {})
    }

    pub(crate) fn finish_session_with<F>(
        &self,
        token: PreviewSessionToken,
        cleanup_current_output: F,
    ) -> bool
    where
        F: FnOnce(),
    {
        let _transition_guard = self.shared.transition_lock.lock().unwrap();
        let _enqueue_guard = self.shared.enqueue_lock.lock().unwrap();
        if !self.is_current(token) {
            return false;
        }
        cleanup_current_output();
        *self.shared.play_state.lock().unwrap() = DawPlayState::Idle;
        self.shared.sink.lock().unwrap().take();
        *self.shared.position.lock().unwrap() = None;
        true
    }

    /// UI の stop 操作用。preview 固有 state と通常再生 state を同じ transition 内で片付ける。
    pub(crate) fn stop_playback(&self) -> DawPlayState {
        let transition_guard = self.shared.transition_lock.lock().unwrap();
        let enqueue_guard = self.shared.enqueue_lock.lock().unwrap();
        let previous = {
            let mut state = self.shared.play_state.lock().unwrap();
            let previous = *state;
            *state = DawPlayState::Idle;
            previous
        };
        let previous_sink = if previous == DawPlayState::Preview {
            self.shared.session.fetch_add(1, Ordering::AcqRel);
            self.shared.sink.lock().unwrap().take()
        } else {
            None
        };
        *self.shared.position.lock().unwrap() = None;

        drop(enqueue_guard);
        drop(transition_guard);
        if let Some(sink) = previous_sink {
            sink.stop();
        }
        previous
    }
}

#[cfg(test)]
mod tests;
