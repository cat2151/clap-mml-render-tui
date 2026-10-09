//! 1 小節 16 step の入力。セルは note on の step と、その音長（step 数）・velocity を持つ。

use crate::DRUM_STEPS;

/// kit ごとに切り替えて編集できる pattern の数。番号は 0 始まり。
pub const PATTERN_COUNT: usize = 16;
/// ON にしたセルの velocity。
pub const DEFAULT_VELOCITY: u8 = 127;
pub(crate) const MAX_VELOCITY: u8 = 127;
pub(crate) const MIDI_NOTES: usize = 128;

/// 1 つの打点。`steps` は 1..=[`DRUM_STEPS`]。小節の終わりを越えてよい。`velocity` は 1..=127。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrumHit {
    pub note: u8,
    pub step: usize,
    pub steps: u8,
    pub velocity: u8,
}

/// `steps` が 0 のセルは OFF。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Cell {
    steps: u8,
    velocity: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrumPattern {
    cells: [[Cell; DRUM_STEPS]; MIDI_NOTES],
}

impl Default for DrumPattern {
    fn default() -> Self {
        Self {
            cells: [[Cell::default(); DRUM_STEPS]; MIDI_NOTES],
        }
    }
}

impl DrumPattern {
    /// 範囲外の note / step は捨て、音長は 1..=[`DRUM_STEPS`]、velocity は 1..=127 に丸める。
    /// 同じセルは後勝ち。
    pub fn from_hits(hits: impl IntoIterator<Item = DrumHit>) -> Self {
        let mut pattern = Self::default();
        for hit in hits {
            pattern.set(hit);
        }
        pattern
    }

    /// note・step の昇順。
    pub fn hits(&self) -> impl Iterator<Item = DrumHit> + '_ {
        (0..MIDI_NOTES as u8)
            .flat_map(move |note| (0..DRUM_STEPS).filter_map(move |step| self.hit(note, step)))
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().flatten().all(|cell| cell.steps == 0)
    }

    /// ON のセルの打点。OFF・範囲外は None。
    pub fn hit(&self, note: u8, step: usize) -> Option<DrumHit> {
        self.cells
            .get(usize::from(note))
            .and_then(|row| row.get(step))
            .filter(|cell| cell.steps > 0)
            .map(|cell| DrumHit {
                note,
                step,
                steps: cell.steps,
                velocity: cell.velocity,
            })
    }

    /// ON のセルの音長。OFF・範囲外は None。
    pub fn length(&self, note: u8, step: usize) -> Option<u8> {
        self.hit(note, step).map(|hit| hit.steps)
    }

    /// 範囲外の note / step は無視する。
    pub(crate) fn set(&mut self, hit: DrumHit) {
        if let Some(cell) = self.cell_mut(hit.note, hit.step) {
            *cell = Cell {
                steps: hit.steps.clamp(1, DRUM_STEPS as u8),
                velocity: hit.velocity.clamp(1, MAX_VELOCITY),
            };
        }
    }

    /// セルを OFF にする。範囲外の note / step は無視する。
    pub(crate) fn clear(&mut self, note: u8, step: usize) {
        if let Some(cell) = self.cell_mut(note, step) {
            *cell = Cell::default();
        }
    }

    fn cell_mut(&mut self, note: u8, step: usize) -> Option<&mut Cell> {
        self.cells
            .get_mut(usize::from(note))
            .and_then(|row| row.get_mut(step))
    }
}

#[cfg(test)]
mod tests;
