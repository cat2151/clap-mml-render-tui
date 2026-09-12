//! `C` キー = chord 行への往復ジャンプ。
//!
//! トグルにしてある。chord 行にいるときの `C` は直前の行へ戻る（行きだけだと戻りに `j` の
//! 連打が要り、往復が非対称になる）。直前の行が無いときは最初の演奏 track へ戻すので、
//! `C` が無反応で終わることは無い。小節（`cursor_measure`）は動かさない。chord 行の同じ小節が
//! カーソル行の生成元なので、ずれると対応が読めなくなる。

use super::super::super::{DawApp, CHORD_TRACK, FIRST_PLAYABLE_TRACK};

impl DawApp {
    /// `C`: chord 行へ跳ぶ。既に chord 行にいるなら跳ぶ前の行へ戻る。
    /// MML overlay からの移送（`mml_overlay_glue`）も同じ関数を通す（跳び方を 2 つ持つと
    /// 「戻り先を覚える」の有無が経路で食い違う）。
    pub(crate) fn jump_between_chord_row_and_cursor_track(&mut self) {
        let destination = if self.editor.cursor_track == CHORD_TRACK {
            self.chord_jump_return_destination()
        } else {
            self.editor.chord_jump_return_track = Some(self.editor.cursor_track);
            CHORD_TRACK
        };
        if destination == self.editor.cursor_track {
            return;
        }
        self.editor.cursor_track = destination;
        // `j` / `k` と同じく preview が追従する。chord 行は演奏されないので、跳んだ側では止まる。
        self.preview_current_target_if_stopped(false);
    }

    /// chord 行から戻る先。覚えている行がグリッドから消えている（track が減った等）場合も
    /// 含め、必ず有効な行 index を返す。
    fn chord_jump_return_destination(&mut self) -> usize {
        let remembered = self
            .editor
            .chord_jump_return_track
            .take()
            .filter(|track| *track >= FIRST_PLAYABLE_TRACK && *track < self.editor.tracks);
        remembered.unwrap_or_else(|| FIRST_PLAYABLE_TRACK.min(self.editor.tracks - 1))
    }
}
