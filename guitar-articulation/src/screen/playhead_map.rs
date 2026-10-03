//! 演奏の秒から、鳴っている列とイベント列の行を引く表。列・イベント列を作り直すたびに作り、描画では引くだけにする。

use super::Take;
use crate::notes::is_note_on;
use crate::TimedMidiEvent;

/// 秒の比較の許し幅。イベント列の秒と列の秒は同じ計算から来るので、ほぼ一致する。
const SECONDS_EPSILON: f64 = 1e-9;

/// 両方の版の表。
#[derive(Default)]
pub(super) struct PlayheadMap {
    plain: TakeMap,
    converted: TakeMap,
}

impl PlayheadMap {
    /// `*_starts` は列ごとの、その版で列の音がいちばん早く鳴る秒（`column_on_seconds` と同じ値）。
    pub(super) fn new(
        plain_starts: &[Option<f64>],
        plain_events: &[TimedMidiEvent],
        converted_starts: &[Option<f64>],
        converted_events: &[TimedMidiEvent],
    ) -> Self {
        PlayheadMap {
            plain: TakeMap::new(plain_starts, plain_events),
            converted: TakeMap::new(converted_starts, converted_events),
        }
    }

    fn take(&self, take: Take) -> &TakeMap {
        match take {
            Take::Plain => &self.plain,
            Take::Converted => &self.converted,
        }
    }

    /// `take` の演奏の頭から `seconds` 秒の時点で鳴っている列。最初の音より前は `None`。
    pub(super) fn column(&self, take: Take, seconds: f64) -> Option<usize> {
        self.take(take).column(seconds)
    }

    /// `take` のイベント列で、列 `column` のいちばん早い note on（同時刻の KS を含む）の行。
    pub(super) fn row(&self, take: Take, column: usize) -> Option<usize> {
        self.take(take).rows.get(column).copied().flatten()
    }
}

/// 1 つの版の表。
#[derive(Default)]
struct TakeMap {
    /// 列の音がいちばん早く鳴る秒の順に、その秒と「その秒までに鳴り始めた列のうち最も後ろの列」。
    starts: Vec<(f64, usize)>,
    /// 列ごとの、イベント列での note on の行。
    rows: Vec<Option<usize>>,
}

impl TakeMap {
    fn new(starts: &[Option<f64>], events: &[TimedMidiEvent]) -> Self {
        let mut ordered: Vec<(f64, usize)> = starts
            .iter()
            .enumerate()
            .filter_map(|(column, seconds)| seconds.map(|seconds| (seconds, column)))
            .collect();
        ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut latest = 0;
        for entry in &mut ordered {
            latest = latest.max(entry.1);
            entry.1 = latest;
        }

        let mut note_ons: Vec<(f64, usize)> = events
            .iter()
            .enumerate()
            .filter(|(_, event)| is_note_on(&event.message))
            .map(|(index, event)| (event.seconds, index))
            .collect();
        note_ons.sort_by(|a, b| a.0.total_cmp(&b.0));
        let rows = starts
            .iter()
            .map(|seconds| first_note_on_at(&note_ons, (*seconds)?))
            .collect();
        TakeMap {
            starts: ordered,
            rows,
        }
    }

    fn column(&self, seconds: f64) -> Option<usize> {
        let count = self.starts.partition_point(|&(on, _)| on <= seconds);
        count.checked_sub(1).map(|last| self.starts[last].1)
    }
}

/// 秒の順の `note_ons` から、`seconds` に鳴る note on のうちイベント列でいちばん前の行。
fn first_note_on_at(note_ons: &[(f64, usize)], seconds: f64) -> Option<usize> {
    let from = note_ons.partition_point(|&(on, _)| on < seconds - SECONDS_EPSILON);
    note_ons[from..]
        .iter()
        .take_while(|&&(on, _)| on <= seconds + SECONDS_EPSILON)
        .map(|&(_, index)| index)
        .min()
}

#[cfg(test)]
mod tests;
