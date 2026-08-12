//! bass 行の step セル上の mouse wheel で、ベースラインを差し替える。
//!
//! wheel は型（[`BassPattern`]）を1つずつ送るだけ。アルペジオ（[`crate::arpeggio`]）と
//! 違って生成は決定的なので、同じ型へ戻れば同じ譜面が出る。
//!
//! 型は名前の並んだ list なので、送りは [`ListDirection`] に従って下で次、上で前。
//! 並びは密度が上がる順だが、上下の意味を決めているのは密度ではなく list であること。
//!
//! 音高は現在のコードの bass 音から導出されるため chord mode 中しか成立しない。

use std::time::Instant;

use cmrt_arpeggiator::{generate_bass_line, BassPattern, BASS_ROOT_VOICE};

use crate::{log_line, GridSequencerScreen, LaneAddress, ListDirection, BASS_ROW, GRID_STEPS};

impl GridSequencerScreen {
    /// bass 行の step セル上の wheel。型を1つ送ってベースラインを引き直す。
    pub(crate) fn cycle_bass_line(&mut self, direction: ListDirection) {
        if !self.state.bass_line_is_available() {
            return;
        }
        let pattern = self.advance_bass_pattern(direction);
        // 引いた結果がたまたま同じ譜面でも、カーソルと表示は送った型に合わせる。
        self.last_bass = Some(pattern);
        let notes = generate_bass_line(pattern, GRID_STEPS);
        let snapshot = self.capture_undo();
        if self.state.apply_bass_line(&notes) {
            self.begin_manual_edit(crate::CycleRandomItem::Arp);
            self.commit_undo(snapshot);
        }
        // 譜面が変わらなかったときも、送った型は必ず聴かせる。
        self.preview_whole_bass_note(pattern);
        log_line(&format!(
            "grid-sequencer: bass-line pattern={}",
            pattern.label()
        ));
    }

    /// 1meas 伸ばす型に限り、小節頭を待たずに root を鳴らし始める。
    ///
    /// 他の型は次の音まで最大2 step（230ms）しか待たないので、そのままで十分聴こえる。
    fn preview_whole_bass_note(&mut self, pattern: BassPattern) {
        if pattern != BassPattern::Whole {
            return;
        }
        let scheduled = self
            .state
            .preview_lane_now(LaneAddress::new(BASS_ROW, BASS_ROOT_VOICE), Instant::now());
        self.send_scheduled(&scheduled);
    }

    /// カーソルを1つ送って、次に適用する型を返す。
    ///
    /// カーソルはリストの手前にある扱いで始める。初回の down は先頭（`Whole`）、初回の
    /// up は末尾（`SixteenthOctave`）になる。
    fn advance_bass_pattern(&mut self, direction: ListDirection) -> BassPattern {
        let next = match (self.bass_pattern, direction) {
            (Some(current), ListDirection::Next) => current.next(),
            (Some(current), ListDirection::Prev) => current.previous(),
            (None, ListDirection::Next) => BassPattern::default(),
            (None, ListDirection::Prev) => BassPattern::default().previous(),
        };
        self.bass_pattern = Some(next);
        next
    }

    /// 譜面を作り直すとき（`t` キーの track 数切替）にカーソルを捨てる。
    pub(crate) fn reset_bass_pattern(&mut self) {
        self.bass_pattern = None;
        self.last_bass = None;
    }

    /// 直近に適用した型。NOTE grid のタイトルへ出す。
    pub fn last_bass(&self) -> Option<BassPattern> {
        self.last_bass
    }
}

#[cfg(test)]
mod tests;
