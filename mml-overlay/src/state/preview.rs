//! 入力中の発音単位と、現在行の再演奏を syntax ごとの変換経路へ流す。

use std::time::Instant;

use ratatui_textarea::DataCursor;

use crate::cursor_notes::{
    notes_at_cursor, notes_at_cursor_with_chord_chart_context, notes_at_cursor_with_chord_context,
    CursorNotes,
};
use crate::line_play::{
    chord_chart_line_events, chord_line_events, line_events, LinePerformance, LineStatus,
};
use crate::NOTE_ON;

use super::{MmlOverlay, MmlOverlayAction, MmlOverlaySyntax, NoteRequest, PatchChange};

impl MmlOverlay<'_> {
    /// カーソルのある行をまるごと鳴らす。
    ///
    /// 打鍵の 1 音は行の演奏に飲み込まれるので、その記録は落とす。ここで note off を
    /// 組み立てないのは、[`MmlOverlayAction::PlayLine`] 自体が「鳴っているものを
    /// 止めてから積む」の意味だから。止めるのは受け取る側の 1 か所だけが行う。
    pub(super) fn play_current_line(&mut self, patch: PatchChange) -> MmlOverlayAction {
        let (status, performance) = self.current_line_performance();
        self.line_status = status;
        self.forget_cursor_unit();
        MmlOverlayAction::PlayLine {
            patch,
            program: self.play_settings.program(performance),
        }
    }

    /// 現在行を syntax 固有の経路で変換する。`Ctrl+Space` と patch selector の
    /// replay/候補移動はこの 1 か所を共有し、別言語への fallback を作らない。
    pub(super) fn current_line_performance(&self) -> (LineStatus, LinePerformance) {
        match &self.syntax {
            MmlOverlaySyntax::Mml => line_events(self.current_line()),
            MmlOverlaySyntax::Chord(Some(context)) => chord_line_events(
                self.current_line(),
                &context.chord_init,
                &context.track_directive,
                &context.mml_prefix,
            ),
            MmlOverlaySyntax::Chord(None) => (LineStatus::Idle, LinePerformance::silent()),
            MmlOverlaySyntax::ChordChart(context) => {
                chord_chart_line_events(self.current_line(), context.key_token.as_deref())
            }
        }
    }

    /// カーソルのある発音単位を調べ、直前と別の単位になっていれば鳴らす。
    ///
    /// 文字を打ったときもカーソルを動かしたときも同じ判定を通るので、
    /// 「← で戻ったらそこの音がまた鳴る」が特別扱いなしに成り立つ。同じ単位の
    /// 内側で動くあいだは鳴らし直さない。
    pub(super) fn refresh(&mut self, _now: Instant) -> MmlOverlayAction {
        let notes = self.notes_at_cursor();
        if notes == self.last_notes {
            return MmlOverlayAction::Continue;
        }
        self.last_notes.clone_from(&notes);
        let Some((_, notes)) = notes else {
            return MmlOverlayAction::Continue;
        };
        MmlOverlayAction::Send(self.start_notes(&notes))
    }

    pub(super) fn notes_at_cursor(&self) -> Option<(usize, CursorNotes)> {
        let DataCursor(row, column) = self.textarea.cursor();
        let notes = match &self.syntax {
            MmlOverlaySyntax::Mml => notes_at_cursor(self.current_line(), column),
            MmlOverlaySyntax::Chord(Some(context)) => notes_at_cursor_with_chord_context(
                self.current_line(),
                column,
                &context.chord_init,
                &context.track_directive,
                &context.mml_prefix,
            ),
            MmlOverlaySyntax::Chord(None) => None,
            MmlOverlaySyntax::ChordChart(context) => notes_at_cursor_with_chord_chart_context(
                self.current_line(),
                column,
                context.key_token.as_deref(),
            ),
        };
        notes.map(|notes| (row, notes))
    }

    /// この発音単位の note on。前の音を止めるのは受け取る側の仕事。
    pub(super) fn start_notes(&mut self, notes: &CursorNotes) -> NoteRequest {
        crate::log_line(format!(
            "action=mml-overlay-note-on pitches={:?} gate_ms={}",
            notes.pitches,
            notes.duration.as_millis()
        ));
        self.sounding.clone_from(&notes.pitches);
        self.sounding_from_chord = notes.from_chord;
        let messages = notes
            .pitches
            .iter()
            .map(|pitch| [NOTE_ON, *pitch, notes.velocity])
            .collect();
        NoteRequest {
            messages,
            duration: notes.duration,
        }
    }

    pub(super) fn current_line(&self) -> &str {
        self.textarea
            .lines()
            .get(self.cursor_row())
            .map_or("", String::as_str)
    }

    pub(super) fn cursor_row(&self) -> usize {
        let DataCursor(row, _) = self.textarea.cursor();
        row
    }

    /// カーソル同一性の記録ごと捨てる。行の演奏後に同じ音へ戻っても再発音させる。
    pub(super) fn forget_cursor_unit(&mut self) {
        self.last_notes = None;
        self.forget_sounding();
    }

    /// 表示の記録を捨てる。実際の音を止めるのは action を受け取る sender。
    pub(super) fn forget_sounding(&mut self) {
        self.sounding.clear();
        self.sounding_from_chord = false;
    }
}
