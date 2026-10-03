//! 列（同時刻の note on のまとまり）と時刻の対応。

use super::{GuitarArticulationScreen, Take};
use crate::TimedMidiEvent;

impl GuitarArticulationScreen {
    /// その版で、列の音がいちばん早く鳴る秒。汚しが ON なら Articulated はずらした後の秒。
    pub(crate) fn column_on_seconds(&self, column: usize, take: Take) -> Option<f64> {
        let mut in_column = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, note)| note.column == column);
        if take == Take::Plain || self.humanized.is_empty() {
            return in_column.next().map(|(_, note)| note.on_seconds);
        }
        in_column
            .map(|(i, _)| self.humanized[i].on_seconds)
            .reduce(f64::min)
    }

    /// カーソル列の音だけを 0 秒から鳴らすイベント列。
    pub fn column_events(&self, take: Take) -> Vec<TimedMidiEvent> {
        crate::column_events(
            &self.notes,
            &self.articulated,
            &self.sounding_rules(),
            self.cursor,
            take,
        )
    }

    /// 列の数（同時刻の note on のまとまりの数）。
    pub fn column_count(&self) -> usize {
        self.notes.last().map_or(0, |note| note.column + 1)
    }

    /// 全体の演奏が鳴っている間は、その版と演奏の頭からの秒を渡す。鳴っていなければ `None`。
    pub fn set_playhead(&mut self, playhead: Option<(Take, f64)>) {
        self.playhead = playhead;
    }

    /// 全体の演奏でいま鳴っている列。最初の音より前と、鳴っていない間は `None`。
    pub(crate) fn playhead_column(&self) -> Option<usize> {
        let (take, seconds) = self.playhead?;
        self.playhead_map.column(take, seconds)
    }

    /// 全体の演奏でいま鳴っている列の、`take` のイベント列での note on の行。鳴っている列が無ければ `None`。
    pub(crate) fn playhead_row(&self, take: Take) -> Option<usize> {
        self.playhead_map.row(take, self.playhead_column()?)
    }

    /// 列ごとの [`Self::column_on_seconds`]。音を 1 回なめて求める。
    pub(super) fn column_starts(&self, take: Take) -> Vec<Option<f64>> {
        let mut starts = vec![None; self.column_count()];
        let written = take == Take::Plain || self.humanized.is_empty();
        for (i, note) in self.notes.iter().enumerate() {
            let slot: &mut Option<f64> = &mut starts[note.column];
            if written {
                slot.get_or_insert(note.on_seconds);
            } else {
                let on = self.humanized[i].on_seconds;
                *slot = Some(slot.map_or(on, |earliest| earliest.min(on)));
            }
        }
        starts
    }
}
