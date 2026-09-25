//! 行の演奏をどの instance で鳴らすか。
//!
//! 音色を変える行は、鳴っている bank とは別の bank の instance へ読み込んで鳴らす。
//! 読み込みの間も鳴っている bank は前の行の release を描き続けるので、切り替えで
//! 音が段差で切れたり無音が挟まったりしない。

use std::time::Instant;

use super::super::live_patch::LivePatch;
use super::super::sink::SoundSink;
use super::super::{log_error, log_line};
use super::{PatchState, Voice};

impl Voice {
    /// 行を鳴らす instance を決め、そこへこの音色を用意する。
    ///
    /// いまの instance がこの音色なら何もしない。違えば、もう一方の bank の instance を使う。
    /// そちらに音色が無ければ、**鳴っている bank を止めずに**読み込む（先読みの経路）。
    /// 同じ instance で読み直すと、読み込みの間その bank の render が止まり、前の行の音が
    /// 段差で切れたうえ無音が挟まる。何も鳴っていないときと、もう一方の bank が無いときは
    /// いまの instance で [`Self::prepare`] する。
    pub(in crate::sender) fn prepare_line(
        &mut self,
        sink: &impl SoundSink,
        patch: &LivePatch,
    ) -> Result<(), String> {
        if self.is_patch_ready(self.line_instance, patch) {
            return Ok(());
        }
        let standby = sink.standby_instance_of(self.line_instance);
        if let Some(standby) = standby.filter(|&standby| self.is_patch_ready(standby, patch)) {
            self.line_instance = standby;
            return Ok(());
        }
        let nothing_to_keep = self.sounding.is_silent() && !self.sounding.needs_hard_stop();
        match standby.filter(|_| !nothing_to_keep) {
            Some(standby) => {
                self.prepare_standby(sink, standby, patch)?;
                self.line_instance = standby;
                Ok(())
            }
            // 途切れさせる音が無いなら、その場で読み込むほうが速い。
            None => self.prepare(sink, self.line_instance, patch),
        }
    }

    /// 行の音色がもう用意できているか（いまの instance か、もう一方の bank の instance に）。
    pub(in crate::sender) fn is_line_patch_ready(
        &self,
        sink: &impl SoundSink,
        patch: &LivePatch,
    ) -> bool {
        self.is_patch_ready(self.line_instance, patch)
            || sink
                .standby_instance_of(self.line_instance)
                .is_some_and(|standby| self.is_patch_ready(standby, patch))
    }

    fn prepare_standby(
        &mut self,
        sink: &impl SoundSink,
        instance_id: u8,
        patch: &LivePatch,
    ) -> Result<(), String> {
        let fields = patch.log_fields();
        log_line(format!(
            "action=mml-overlay-prepare event=start command_id={} instance={instance_id} {fields} \
             standby=true",
            self.command_id,
        ));
        let started_at = Instant::now();
        // 読み込みが失敗・中断したら中身は分からない。成功するまで準備済みにしない。
        self.patches.remove(&instance_id);
        let result = sink.prepare_standby_patch(instance_id, patch);
        match &result {
            Ok(()) => {
                self.patches.insert(
                    instance_id,
                    PatchState {
                        current: patch.clone(),
                        ready: true,
                    },
                );
                log_line(format!(
                    "action=mml-overlay-prepare event=success command_id={} instance={instance_id} {fields} \
                     standby=true elapsed_ms={}",
                    self.command_id,
                    started_at.elapsed().as_millis()
                ));
            }
            Err(error) => log_error(format!(
                "action=mml-overlay-prepare event=error command_id={} instance={instance_id} {fields} \
                 standby=true elapsed_ms={} error=\"{error}\"",
                self.command_id,
                started_at.elapsed().as_millis()
            )),
        }
        result
    }
}
