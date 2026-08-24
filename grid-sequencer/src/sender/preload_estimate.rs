//! auto random の待機 bank 先読みで使う、catalog 重み付き ETA。

use std::time::{Duration, Instant};

/// 今回選ばれた patch 群と実測時間から、残りのロード時間を見積もる。
///
/// `weights_ms` は catalog の2回目ロード時間。絶対値は最初の見積もりに使い、
/// 1件完了した後は「実測時間 ÷ 完了済みの重み」で今回の環境に合わせて補正する。
/// したがって、最大値を1.0にする正規化と同じ比率を保ちつつ、浮動小数点の重みを
/// 状態として持たずに済む。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridPreloadEstimate {
    weights_ms: Vec<u64>,
    started: usize,
    completed: usize,
    measured_elapsed: Duration,
    current_started_at: Option<Instant>,
}

impl GridPreloadEstimate {
    pub(crate) fn new(weights_ms: Vec<u64>) -> Self {
        Self {
            weights_ms: weights_ms.into_iter().map(|weight| weight.max(1)).collect(),
            started: 0,
            completed: 0,
            measured_elapsed: Duration::ZERO,
            current_started_at: None,
        }
    }

    pub(crate) fn begin_step(&mut self, now: Instant) {
        if self.started == self.completed && self.started < self.weights_ms.len() {
            self.started += 1;
            self.current_started_at = Some(now);
        }
    }

    /// 実際にロードを始めた1件だけを完了扱いにする。キャンセル後に遅れて届いた
    /// 結果や重複通知で、進捗が総数を追い越さないようにする。
    pub(crate) fn record_step(&mut self, elapsed: Duration) -> bool {
        if self.completed >= self.started || self.completed >= self.weights_ms.len() {
            return false;
        }
        self.completed += 1;
        self.measured_elapsed = self.measured_elapsed.saturating_add(elapsed);
        self.current_started_at = None;
        true
    }

    pub(crate) fn total(&self) -> usize {
        self.weights_ms.len()
    }

    pub(crate) fn completed(&self) -> usize {
        self.completed
    }

    pub(crate) fn current_instance(&self) -> Option<usize> {
        (self.started > self.completed).then_some(self.started)
    }

    pub(crate) fn measured_elapsed(&self) -> Duration {
        self.measured_elapsed
    }

    pub(crate) fn eta(&self, now: Instant) -> Duration {
        let completed_weight = sum_weights(&self.weights_ms[..self.completed]);
        let remaining_weight = sum_weights(&self.weights_ms[self.completed..]);
        if remaining_weight == 0 {
            return Duration::ZERO;
        }
        if completed_weight == 0 || self.measured_elapsed.is_zero() {
            return Duration::from_millis(remaining_weight)
                .saturating_sub(self.current_elapsed(now));
        }
        let eta_micros = self
            .measured_elapsed
            .as_micros()
            .saturating_mul(u128::from(remaining_weight))
            / u128::from(completed_weight);
        Duration::from_micros(u64::try_from(eta_micros).unwrap_or(u64::MAX))
            .saturating_sub(self.current_elapsed(now))
    }

    fn current_elapsed(&self, now: Instant) -> Duration {
        self.current_started_at
            .map(|started| now.saturating_duration_since(started))
            .unwrap_or_default()
    }
}

fn sum_weights(weights: &[u64]) -> u64 {
    weights.iter().copied().fold(0, u64::saturating_add)
}

#[cfg(test)]
mod tests;
