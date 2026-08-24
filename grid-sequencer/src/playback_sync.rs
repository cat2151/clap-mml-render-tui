//! 出力 drop で止まったサーバーの musical clock と表示クロックを同期する。
//!
//! サーバーの出力リングが空になると、オーディオ callback は無音を出して
//! `underrun_frames` を増やす。その間、レンダー側の timeline sample clock は進まない。
//! wall clock のまま表示すると、この無音フレームの累積ぶんだけ表示が先行するため、
//! timeline 開始後の underrun を wall clock から差し引いて表示時刻を作る。

use std::time::{Duration, Instant};

use crate::GRID_STEPS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MeasureSyncCheck {
    pub(crate) measure: u64,
    pub(crate) dropped_frames: u64,
    pub(crate) dropped: Duration,
    pub(crate) display_lateness: Duration,
}

#[derive(Debug, Default)]
pub(crate) struct PlaybackSync {
    last_server_underrun_frames: u64,
    dropped_frames: u64,
    last_audible_now: Option<Instant>,
    last_checked_measure: Option<u64>,
}

impl PlaybackSync {
    pub(crate) fn restart(&mut self, now: Instant, server_underrun_frames: u64) {
        self.last_server_underrun_frames = server_underrun_frames;
        self.dropped_frames = 0;
        self.last_audible_now = Some(now);
        self.last_checked_measure = None;
    }

    /// wall clock から、この timeline の開始後に出力された無音フレームぶんを引く。
    ///
    /// counter が一度に大きく更新された場合も表示時刻は巻き戻さない。既に見せた列を
    /// 逆再生せず、その位置で止めて音側が追いつくのを待つ。
    pub(crate) fn audible_now(
        &mut self,
        wall_now: Instant,
        server_underrun_frames: u64,
        sample_rate: f64,
    ) -> Instant {
        if server_underrun_frames >= self.last_server_underrun_frames {
            self.dropped_frames = self.dropped_frames.saturating_add(
                server_underrun_frames.saturating_sub(self.last_server_underrun_frames),
            );
        }
        self.last_server_underrun_frames = server_underrun_frames;

        let estimated = wall_now
            .checked_sub(frames_duration(self.dropped_frames, sample_rate))
            .unwrap_or(wall_now);
        let audible = self
            .last_audible_now
            .map_or(estimated, |previous| previous.max(estimated));
        self.last_audible_now = Some(audible);
        audible
    }

    pub(crate) fn check_measure(
        &mut self,
        displayed_ordinal: Option<u64>,
        display_lateness: Duration,
        sample_rate: f64,
    ) -> Option<MeasureSyncCheck> {
        let measure = displayed_ordinal? / GRID_STEPS as u64;
        if self.last_checked_measure == Some(measure) {
            return None;
        }
        self.last_checked_measure = Some(measure);
        Some(MeasureSyncCheck {
            measure,
            dropped_frames: self.dropped_frames,
            dropped: frames_duration(self.dropped_frames, sample_rate),
            display_lateness,
        })
    }
}

fn frames_duration(frames: u64, sample_rate: f64) -> Duration {
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Duration::ZERO;
    }
    Duration::from_secs_f64(frames as f64 / sample_rate)
}

#[cfg(test)]
mod tests;
