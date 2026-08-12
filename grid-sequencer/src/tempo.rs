use std::time::Instant;

use cmrt_tui_core::bpm::{BpmInputAction, BpmMode};
use crossterm::event::KeyEvent;

use crate::{AppliedTempo, GridSequencerScreen};

impl GridSequencerScreen {
    pub(crate) fn handle_bpm_input_key(&mut self, key: KeyEvent, now: Instant) {
        let Some(input) = self.bpm_input.as_mut() else {
            return;
        };
        match input.handle_key(key) {
            BpmInputAction::Continue => {}
            BpmInputAction::Cancel => self.bpm_input = None,
            BpmInputAction::Apply(mode) => self.apply_bpm_mode(mode, now),
            BpmInputAction::ApplyAuto(range) => {
                if let Some(range) = range {
                    self.bpm_range = range;
                }
                self.apply_bpm_mode(BpmMode::Auto(self.bpm_range.sample()), now);
            }
        }
    }

    /// 次のコード進行1周ぶんのテンポを予約しておく。毎フレーム呼んでよい。
    ///
    /// 予約は `GridState` がコード進行の頭で適用する（[`crate::state`]）。そこでは演奏を
    /// 止めず、絶対 musical time も繋いだままテンポだけ乗り換えるので、周をまたいでも
    /// 音が飛ばない。**単位は小節ではなく進行1周**で、進行の途中でテンポが動くと同じ
    /// コード進行が別々の速さで演奏されてフレーズが繋がらない。chord mode を使って
    /// いない間だけ grid 1周が単位になる。
    /// 手動モード中と、幅のない範囲（既定の `BPM` 固定）と、BPM を1周ごとの random から
    /// 外しているときは何もしない。
    pub(crate) fn arm_next_cycle_bpm(&mut self) {
        if !self.cycle_random.bpm
            || self.bpm_mode.auto_target().is_none()
            || self.bpm_range.is_fixed()
            || self.state.has_armed_cycle_bpm()
        {
            return;
        }
        self.state.arm_next_cycle_bpm(self.bpm_range.sample());
    }

    /// コード進行の頭で乗り換えたテンポを表示用の `bpm_mode` へ取り込み、サーバーの
    /// tempo map へも同じ変化点を積む。
    ///
    /// サーバーへ送らないと、クライアントのステップ間隔だけが変わって CLAP transport の
    /// テンポは古いまま残る（tempo-sync する delay/LFO が追従しない）。
    pub(crate) fn absorb_applied_cycle_bpm(&mut self) {
        let Some(applied) = self.state.take_applied_cycle_bpm() else {
            return;
        };
        self.bpm_mode = BpmMode::Auto(applied.bpm);
        self.send_tempo_change(applied);
        crate::log_line(&format!(
            "grid-sequencer: bpm cycle-redraw value={} at_seconds={:.6} range={}",
            applied.bpm,
            applied.at_timeline_seconds,
            self.bpm_range.label()
        ));
    }

    /// テンポ変化点をサーバーの tempo map へ積む。**`begin_timeline` は使わないこと。**
    /// あれは音楽的な epoch の張り直しで、プラグインの状態もサンプルクロックの原点も
    /// 巻き添えで初期化される。
    fn send_tempo_change(&self, applied: AppliedTempo) {
        if let Some(sender) = &self.midi_sender {
            sender.set_live_tempo(self.timeline_id, applied.at_timeline_seconds, applied.bpm);
        }
    }

    /// クロックを走らせる前（入場時）に自動BPMだけ引き直す。
    ///
    /// 呼び出し側が直後に `start_at_bpm` するので、ここではモードを差し替えるだけに留める。
    pub(crate) fn reseed_auto_bpm(&mut self) {
        if self.bpm_mode.auto_target().is_none() || self.bpm_range.is_fixed() {
            return;
        }
        self.bpm_mode = BpmMode::Auto(self.bpm_range.sample());
    }

    /// Ctrl+B / A キーでのテンポ変更を適用する。
    ///
    /// 演奏中は **timeline を張り直さず**、次のステップ境界からの tempo map 追記で
    /// 乗り換える。`restart_timeline`（＝ `BeginTimeline`）はサーバー側で
    /// `reset_all(renderers)` とサンプルクロックの原点戻しを伴うので、鳴っている音が
    /// 切れて演奏が先頭へ飛ぶ。停止中と画面入場時だけが張り直してよい場面。
    fn apply_bpm_mode(&mut self, mode: BpmMode, now: Instant) {
        self.bpm_input = None;
        if self.bpm_mode == mode {
            return;
        }
        self.cancel_mouse_gesture();
        // 周の頭へ向けた古い抽選は、ここで決めたテンポを上書きしてしまうので取り下げる。
        self.state.disarm_next_cycle_bpm();
        let running = self.state.is_running();
        // 表示 BPM が動かない切替（AUTO と同値の MANUAL など）で止まっている演奏を
        // 張り直しても、先頭へ飛ぶだけ損。判定は `bpm_mode` を差し替える前に取る。
        let restart = !running && self.bpm() != mode.bpm();
        self.bpm_mode = mode;
        // 演奏中は絶対に張り直さない。`retempo_from_next_step` は実 BPM が同じなら
        // None を返すので、動かない切替では何も起きない。
        let applied = running
            .then(|| self.state.retempo_from_next_step(mode.bpm()))
            .flatten();
        if let Some(applied) = applied {
            // 音も絶対 musical time も繋いだままテンポだけ乗り換える。
            self.send_tempo_change(applied);
        } else if restart {
            // 停止中（＝まだ鳴っていない）。ここは epoch を張り直してよい。
            self.cancel_cycle_swap();
            let note_offs = self.state.take_reset_messages();
            self.send_scheduled(&note_offs);
            self.restart_timeline(now);
        }
        crate::log_line(&format!(
            "grid-sequencer: bpm mode={} value={} range={} restart={restart} at_seconds={}",
            self.bpm_mode.label(),
            self.bpm(),
            self.bpm_range.label(),
            applied
                .map(|applied| format!("{:.6}", applied.at_timeline_seconds))
                .unwrap_or_else(|| "-".to_string()),
        ));
    }
}

#[cfg(test)]
mod tests;
