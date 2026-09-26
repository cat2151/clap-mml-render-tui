//! 音色（`t`）・EFFECT CHAIN（`x`）を確定したあとの自動演奏の予約。
//!
//! 確定した時点のカーソル小節の render が済んだとき、全体が無音なら `Space` と同じ演奏を
//! 始める。無音でなければ（演奏・preview・起動時の試聴ループのどれかが鳴っている）、
//! または NORMAL 以外の画面にいれば、鳴らさずに予約を捨てる。

use super::startup_audition::measure_render_settled;
use crate::{DawApp, DawMode, DawPlayState};

impl DawApp {
    /// 現在のカーソル小節で自動演奏を予約する。前の予約は置き換える。
    pub(crate) fn reserve_auto_play_after_render(&self) {
        let measure = self.editor.cursor_measure.max(1).min(self.editor.measures);
        *self.playback.auto_play_reservation.lock().unwrap() = Some(measure);
        self.append_log_line(format!("auto-play: reserve meas{measure}"));
    }

    pub(crate) fn cancel_auto_play_reservation(&self) {
        *self.playback.auto_play_reservation.lock().unwrap() = None;
    }

    /// メインループが毎 tick 呼ぶ。予約した小節の render が済んだら 1 回だけ判定する。
    pub(crate) fn pump_auto_play_reservation(&self) {
        let Some(measure) = *self.playback.auto_play_reservation.lock().unwrap() else {
            return;
        };
        if !measure_render_settled(
            &self.cache.lock().unwrap(),
            measure,
            &self.playback_track_gains(),
        ) {
            return;
        }
        self.cancel_auto_play_reservation();
        if self.mode != DawMode::Normal {
            self.append_log_line("auto-play: skip reason=not-normal-mode");
            return;
        }
        let silent = *self.playback.play_state.lock().unwrap() == DawPlayState::Idle
            && self.playback.startup_audition.lock().unwrap().is_none();
        if !silent {
            self.append_log_line("auto-play: skip reason=sounding");
            return;
        }
        self.append_log_line(format!("auto-play: start meas{measure} rendered"));
        self.start_play();
    }
}

#[cfg(test)]
mod tests;
