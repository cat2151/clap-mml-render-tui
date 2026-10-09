//! 打点を差し替えられるループ（[`crate::sender::step_loop`]）を timeline へ張る。

use std::time::{Duration, Instant};

use super::super::sink::SoundSink;
use super::super::step_loop::{StepLoop, StepLoopEdit, StepLoopState};
use super::super::{log_error, log_line};
use super::{
    advance_timeline_id, begin_timeline, send_cycle, LineOutcome, LinePlayback, LOOKAHEAD_SECONDS,
};

impl LinePlayback {
    /// 新しい timeline を張り、打点を差し替えられるループを始める。最初の先読みぶんも積む。
    ///
    /// `Ok` は周の先頭（先読みの遅れ込み）が鳴る実時刻。
    pub(in crate::sender) fn play_step_loop(
        &mut self,
        sink: &impl SoundSink,
        instance_id: u8,
        step_loop: StepLoop,
    ) -> Result<Instant, LineOutcome> {
        self.stop_repeat();
        self.instance_id = instance_id;
        let timeline_id = self.next_timeline_id;
        self.next_timeline_id = advance_timeline_id(timeline_id);
        if !begin_timeline(sink, timeline_id, self.sample_rate_hz) {
            return Err(LineOutcome::Failed);
        }
        let origin = Instant::now();
        let Some(state) = StepLoopState::new(origin, step_loop) else {
            log_error(format!(
                "action=mml-overlay-step-loop event=invalid-loop timeline={timeline_id}"
            ));
            return Err(LineOutcome::Failed);
        };
        log_line(format!(
            "action=mml-overlay-step-loop event=start timeline={timeline_id}"
        ));
        self.step_loop = Some((timeline_id, state));
        match self.pump(sink, origin) {
            Some(LineOutcome::Playing) => Ok(origin + Duration::from_secs_f64(LOOKAHEAD_SECONDS)),
            _ => Err(LineOutcome::Partial),
        }
    }

    /// 走っている打点ループを変える。ループが無ければ何もしない。
    ///
    /// 単発だけはその場で送る。返すのは送ったときだけで、`Partial` は送れずにループを
    /// 捨てたことを表す。
    pub(in crate::sender) fn edit_step_loop(
        &mut self,
        sink: &impl SoundSink,
        edit: StepLoopEdit,
        now: Instant,
    ) -> Option<LineOutcome> {
        let (timeline_id, state) = self.step_loop.as_mut()?;
        let shot = match edit {
            StepLoopEdit::Hits(hits) => {
                state.set_hits(hits);
                return None;
            }
            StepLoopEdit::Horizon(seconds) => {
                state.set_horizon(seconds);
                return None;
            }
            StepLoopEdit::Shot(shot) => shot,
        };
        let events = state.take_shot(now, shot);
        if !send_cycle(sink, *timeline_id, self.instance_id, &events) {
            self.step_loop = None;
            return Some(LineOutcome::Partial);
        }
        Some(LineOutcome::Playing)
    }
}
