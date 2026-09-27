//! `r`（全部引き直し）・`R`（音色据え置きの引き直し）・`x`（note を消す）の、grid 全体への操作。

use std::time::Instant;

use crate::{log_line, CycleRandomItem, GridSequencerContext, GridSequencerScreen};

impl GridSequencerScreen {
    /// grid を丸ごと引き直し、全 instance の patch を差し替える。
    pub(crate) fn randomize(&mut self, now: Instant, ctx: &GridSequencerContext<'_>) {
        let undo = self.capture_undo();
        self.patch_status = ctx.patch_status();
        // 全 instance を差し替えるので、走っている先読みは意味を失う。
        self.cancel_cycle_swap();
        let _note_offs = self.state.randomize_all(now, &[]);
        self.absorb_drawn_phrases(self.state.drawn_phrases());
        // chord mode 中は和音の行だけ poly patch を当て直す（無差別抽選で mono を
        // 引くと和音が潰れるため）。
        self.rechord_after_randomize(now, ctx, true);
        log_line(&format!(
            "grid-sequencer: randomize instances={}",
            self.track_count()
        ));
        self.prepare_connection();
        self.commit_undo(undo);
    }

    /// patch を据え置き、note / patternだけを引き直す。
    ///
    /// 音色ロード（`sender.prepare()`）を走らせないので再生が途切れない。その代わり
    /// `prepare_instances()` の `stop_live_all()` による消音も無いため、鳴っていた音の
    /// note off はここで自分で送る必要がある。
    pub(crate) fn randomize_keeping_patches(
        &mut self,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) {
        let undo = self.capture_undo();
        // 譜面が変わるので、抽選済みの次サイクルは古くなる。走っている先読みごと捨てる。
        self.cancel_cycle_swap_preserving_drain();
        let note_offs = self.state.randomize_keeping_patches(now);
        self.absorb_drawn_phrases(self.state.drawn_phrases());
        log_line(&format!(
            "grid-sequencer: randomize-keep-patch instances={} note_offs={}",
            self.track_count(),
            note_offs.len(),
        ));
        self.send_scheduled(&note_offs);
        self.rechord_after_randomize(now, ctx, false);
        self.commit_undo(undo);
    }

    pub(crate) fn clear_notes(&mut self) {
        let undo = self.capture_undo();
        // `x` は白紙の pattern を保持するという明示操作なので、すでに空でも
        // NOTE の引き直しを止める。
        self.begin_manual_edit(CycleRandomItem::Note);
        self.state.clear_notes();
        self.commit_undo(undo);
    }
}
