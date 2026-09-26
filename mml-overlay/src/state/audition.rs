//! 入力欄の音色 selector（`Ctrl+T`）で、試聴に何を鳴らすかの判断。
//!
//! 鳴らす部分（[`cmrt_patch_select::PatchAuditionSelect`]）はこの判断を知らない。入力欄の仕様は
//! 「カーソル位置の音を鳴らす」なので、その判断は入力欄を持つこちらに置く。

use cmrt_patch_select::PatchAudition;

use crate::cursor_notes::{preview_note, CursorNotes};
use crate::line_play::{LinePerformance, LineStatus};

use super::{MmlOverlay, MmlOverlaySyntax};

impl MmlOverlay<'_> {
    /// 音色一覧のカーソルが動いたときに鳴らすもの。
    ///
    /// repeat が ON なら行全体。音色を↑↓で流しながら同じフレーズを聴き比べるのが
    /// repeat の目的なので、1 音へ落とすとループが途切れて目的を果たさない。
    ///
    /// ただし鳴らす行が無いとき（空行・解釈できない行）は repeat でも 1 音へ戻す。
    /// ループの代わりに無音になると、音色そのものを聴く手段が消えてしまうため。
    ///
    /// カーソルがどの発音単位にも触れていない（行末のコメントの後ろ等）ときは行全体。
    /// 試聴用の 1 音へ落とすと、DAW が生成した行のように音がある行でも単音になる。
    pub(super) fn candidate_audition(&mut self) -> Option<PatchAudition> {
        let (status, performance) = self.current_line_performance();
        if !performance.is_silent()
            && (self.play_settings().repeat || self.notes_at_cursor().is_none())
        {
            return Some(self.line_audition(status, performance));
        }
        self.notes_audition().map(PatchAudition::Notes)
    }

    /// preset を保存して先頭候補が変わったときに鳴らすもの。入力欄では repeat でも
    /// 行全体にせず、カーソル位置の音。
    pub(super) fn preset_audition(&mut self) -> Option<PatchAudition> {
        self.notes_audition().map(PatchAudition::Notes)
    }

    /// 行全体を鳴らすと決めたときの入力欄側の記録。打鍵の 1 音は行の演奏に飲み込まれる。
    fn line_audition(&mut self, status: LineStatus, performance: LinePerformance) -> PatchAudition {
        self.line_status = status;
        self.forget_cursor_unit();
        PatchAudition::Line(performance)
    }

    /// カーソル位置の音。まだ MML が空でも音色は聴きたいので、その場合だけ試聴用の 1 音。
    ///
    /// Chord Chart では試聴用の 1 音へ落とさない（MML の `c` は進行の音ではない）。
    pub(super) fn notes_audition(&mut self) -> Option<CursorNotes> {
        let at_cursor = self.notes_at_cursor();
        self.last_notes.clone_from(&at_cursor);
        let notes = at_cursor.map(|(_, notes)| notes).or_else(|| {
            (!matches!(self.syntax, MmlOverlaySyntax::ChordChart(_)))
                .then(preview_note)
                .flatten()
        })?;
        self.show_sounding(&notes);
        Some(notes)
    }
}
