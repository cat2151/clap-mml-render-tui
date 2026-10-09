//! 打点を差し替えられるループ（[`crate::sender::step_loop`]）を鳴らす入口。

use std::time::Instant;

use super::super::line_playback::LineOutcome;
use super::super::sink::SoundSink;
use super::super::step_loop::{StepLoop, StepLoopEdit};
use super::Voice;

impl Voice {
    /// 鳴っているものを止め、新しい timeline で打点ループを始める。
    ///
    /// 打点が 1 つも無くても始める（時計だけ進め、後から足した打点を鳴らす）。
    /// 返すのは周の先頭が鳴る実時刻。張れなかったら `None`。
    pub(in crate::sender) fn play_step_loop(
        &mut self,
        sink: &impl SoundSink,
        step_loop: StepLoop,
    ) -> Option<Instant> {
        self.stop_before_timeline(sink, "step-loop");
        self.sounding_lines.record(self.line_instance);
        self.server_drops.watch(sink);
        match self
            .line
            .play_step_loop(sink, self.line_instance, step_loop)
        {
            Ok(origin) => {
                self.sounding.begin_timeline();
                Some(origin)
            }
            Err(LineOutcome::Failed) | Err(LineOutcome::Playing) => {
                self.sounding.mark_suspect("step-loop-begin-failed");
                None
            }
            Err(LineOutcome::Partial) => {
                self.sounding.begin_timeline();
                self.sounding.mark_suspect("step-loop-send-incomplete");
                None
            }
        }
    }

    /// 走っている打点ループを変える。単発の送信に失敗したらループを捨て、次の停止を
    /// server 管理の全NoteOffへ倒す。
    pub(in crate::sender) fn edit_step_loop(&mut self, sink: &impl SoundSink, edit: StepLoopEdit) {
        if self.line.edit_step_loop(sink, edit, Instant::now()) == Some(LineOutcome::Partial) {
            self.sounding.mark_suspect("step-loop-shot-send-incomplete");
        }
    }
}
