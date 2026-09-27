//! 起動時自動演奏だけのログ。画面用の進捗・再生条件とは独立して保持する。
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Default)]
pub(crate) struct Progress {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    deadline: Option<Instant>,
    finished: bool,
}

impl Progress {
    pub(crate) fn log(&self, logs: &Arc<Mutex<VecDeque<String>>>, message: impl AsRef<str>) {
        let state = self.state.lock().unwrap();
        if !state.finished {
            crate::append_log_line(logs, format!("起動時自動演奏: {}", message.as_ref()));
        }
    }

    pub(crate) fn schedule(&self, deadline: Instant) {
        self.state.lock().unwrap().deadline = Some(deadline);
    }

    pub(crate) fn poll(
        &self,
        logs: &Arc<Mutex<VecDeque<String>>>,
        playing: bool,
        now: Instant,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        if !state.finished {
            let message = if !playing {
                Some("本演奏の開始前に停止しました。")
            } else if state.deadline.is_some_and(|deadline| now >= deadline) {
                Some("本演奏（Shift+Space相当）を開始しました。")
            } else {
                None
            };
            if let Some(message) = message {
                crate::append_log_line(logs, format!("起動時自動演奏: {message}"));
                state.finished = true;
            }
        }
        state.finished
    }

    pub(crate) fn cancel(&self, logs: &Arc<Mutex<VecDeque<String>>>, reason: &str) {
        let mut state = self.state.lock().unwrap();
        if !state.finished {
            crate::append_log_line(
                logs,
                format!("起動時自動演奏: 自動演奏を取り消しました。理由={reason}"),
            );
            state.finished = true;
        }
    }
}

#[cfg(test)]
mod tests;
