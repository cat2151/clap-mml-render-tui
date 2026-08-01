use std::time::{Duration, Instant};

pub(super) const INITIAL_BUFFER_MULTIPLIER: u16 = 2;
pub(super) const RESTORE_BUFFER_MULTIPLIER: u16 = 4;
/// 梯子の下限。これ以上は薄くしない（＝入場時の厚さ）。
const MIN_BUFFER_MULTIPLIER: u16 = INITIAL_BUFFER_MULTIPLIER;
/// 梯子の上限。サーバー側のリング容量（`MAX_BUFFER_MULTIPLIER`）と同じ値でなければ
/// ならないので、共有 crate の定数をそのまま使う。
const MAX_BUFFER_MULTIPLIER: u16 = cmrt_realtime_play::MAX_LIVE_BUFFER_MULTIPLIER;
const STABLE_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AdaptiveBuffer {
    multiplier: u16,
    underrun_frames: u64,
    last_total_underrun_frames: u64,
    stable_since: Instant,
}

impl AdaptiveBuffer {
    pub(super) fn new(now: Instant, total_underrun_frames: u64) -> Self {
        Self {
            multiplier: INITIAL_BUFFER_MULTIPLIER,
            underrun_frames: 0,
            last_total_underrun_frames: total_underrun_frames,
            stable_since: now,
        }
    }

    pub(super) fn multiplier(self) -> u16 {
        self.multiplier
    }

    /// サーバーが倍率を受け付けなかったときに、直前の値へ戻す。
    ///
    /// 拒否された値を持ったままだと、次の `observe()` がそこからさらに1段上げようとして
    /// 失敗し続ける。古いサーバー相手でも梯子が止まるようにするため巻き戻す。
    pub(super) fn revert(&mut self, multiplier: u16) {
        self.multiplier = multiplier;
    }

    pub(super) fn underrun_frames(self) -> u64 {
        self.underrun_frames
    }

    /// Returns a new multiplier when the server setting must change.
    pub(super) fn observe(&mut self, now: Instant, total_underrun_frames: u64) -> Option<u16> {
        if total_underrun_frames < self.last_total_underrun_frames {
            self.last_total_underrun_frames = total_underrun_frames;
            self.underrun_frames = 0;
            self.stable_since = now;
            return None;
        }

        let new_underruns = total_underrun_frames.saturating_sub(self.last_total_underrun_frames);
        self.last_total_underrun_frames = total_underrun_frames;
        if new_underruns > 0 {
            self.underrun_frames = self.underrun_frames.saturating_add(new_underruns);
            self.stable_since = now;
            if let Some(next) = next_larger(self.multiplier) {
                self.multiplier = next;
                self.underrun_frames = 0;
                return Some(next);
            }
            return None;
        }

        if now.saturating_duration_since(self.stable_since) >= STABLE_INTERVAL {
            if let Some(next) = next_smaller(self.multiplier) {
                self.multiplier = next;
                self.underrun_frames = 0;
                self.stable_since = now;
                return Some(next);
            }
        }
        None
    }
}

/// 梯子を1段上げる。上限に達していたら `None`。
fn next_larger(multiplier: u16) -> Option<u16> {
    (multiplier < MAX_BUFFER_MULTIPLIER).then(|| multiplier.saturating_mul(2))
}

/// 梯子を1段下げる。下限に達していたら `None`。
fn next_smaller(multiplier: u16) -> Option<u16> {
    (multiplier > MIN_BUFFER_MULTIPLIER).then_some(multiplier / 2)
}

#[cfg(test)]
mod tests;
