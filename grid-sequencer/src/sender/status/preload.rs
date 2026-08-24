//! 待機 bank への patch 先読みについて、sender と表示が共有する状態更新。

use std::time::{Duration, Instant};

use crate::sender::GridPreloadEstimate;

use super::{GridConnectionStatus, GridProgress};

impl GridConnectionStatus {
    /// 先読みロードを1件投げた。UI スレッドから、送信の直前に呼ぶ。
    pub(in crate::sender) fn begin_preload_step(&mut self) {
        if let Some(estimate) = &mut self.preload_estimate {
            estimate.begin_step(Instant::now());
        }
    }

    /// 先読みロードを1件終えた。送信スレッドから呼ぶ。
    pub(in crate::sender) fn record_preload_step(&mut self, succeeded: bool, elapsed: Duration) {
        let Some(estimate) = &mut self.preload_estimate else {
            return;
        };
        if !estimate.record_step(elapsed) {
            return;
        }
        self.preload.completed = estimate.completed();
        if !succeeded {
            self.preload_failed = true;
        }
    }

    /// 先読みの集計を初期化する。新しいサイクルの先読みを始める前に呼ぶ。
    pub(in crate::sender) fn reset_preload(&mut self, load_weights_ms: Vec<u64>) {
        let estimate = GridPreloadEstimate::new(load_weights_ms);
        self.preload = GridProgress {
            completed: 0,
            total: estimate.total(),
        };
        self.preload_estimate = Some(estimate);
        self.preload_failed = false;
    }

    /// 完了した結果は次のサイクルまで残す。途中キャンセルなら「load中」に見え続けない
    /// よう、その場で進捗を消す。
    pub(in crate::sender) fn finish_preload(&mut self) {
        if self.preload.completed < self.preload.total {
            self.clear_preload();
        }
    }

    pub(in crate::sender) fn clear_preload(&mut self) {
        self.preload = GridProgress::default();
        self.preload_estimate = None;
        self.preload_failed = false;
    }

    pub fn preload_current_instance(&self) -> Option<usize> {
        self.preload_estimate
            .as_ref()
            .and_then(GridPreloadEstimate::current_instance)
    }

    pub fn preload_eta(&self) -> Option<Duration> {
        self.preload_estimate
            .as_ref()
            .map(|estimate| estimate.eta(Instant::now()))
    }

    pub fn preload_measured_elapsed(&self) -> Option<Duration> {
        self.preload_estimate
            .as_ref()
            .map(GridPreloadEstimate::measured_elapsed)
    }
}

#[cfg(test)]
mod tests;
