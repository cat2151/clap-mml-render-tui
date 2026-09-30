use std::ops::RangeInclusive;

use crate::hammer_pull::single_in_column;
use crate::{Articulation, Note, Rule, RuleTable};

/// スライドの幅の上限（半音）。`Slide_Up_8ST` / `Slide_Down_8ST` が 3 音半。
pub const SLIDE_MAX_SEMITONES: u8 = 7;

/// スライドの sample が在る最低音と最高音。幅 w の `Slide_Up` は `SLIDE_UP_LOWEST_START + w` から、
/// 幅 w の `Slide_Down` は `SLIDE_HIGHEST_PITCH - w` まで。
const SLIDE_LOWEST_PITCH: u8 = 30;
const SLIDE_UP_LOWEST_START: u8 = 31;
const SLIDE_HIGHEST_PITCH: u8 = 88;

/// チョーキング（`Bending_HT` / `_WH` / `_1HT`）の sample が在る音高。
pub const BEND_PITCHES: RangeInclusive<u8> = 30..=90;

/// 列ごとの [`Rule::Slide`] / [`Rule::Choke`] で、前の列からの音程に応じた奏法を決める。
///
/// 対象は単音の列から単音の列へ移る所だけ。音程と音高が sample の範囲に在れば、
/// それまでの奏法（自動ハンマリングの H/P を含む）を上書きする。範囲外ならそのまま残す。
/// - [`Rule::Slide`]: 上行 1〜7 半音で `Slide_Up`、下行 1〜7 半音で `Slide_Down`。
/// - [`Rule::Choke`]: 上行 1 / 2 / 3 半音で `Bending_HT` / `_WH` / `_1HT`。
///
/// 鳴らす音高はどちらも行き先の音（sfz が下や前の音高から滑らせる）。
pub fn apply_glide_rules(notes: &[Note], rules: &RuleTable, articulations: &mut [Articulation]) {
    for (i, note) in notes.iter().enumerate() {
        let chosen = if rules.is_on(note.column, Rule::Slide) {
            interval_from_previous(notes, i).and_then(|d| slide(note.pitch, d))
        } else if rules.is_on(note.column, Rule::Choke) {
            interval_from_previous(notes, i).and_then(|d| bend(note.pitch, d))
        } else {
            None
        };
        if let Some(articulation) = chosen {
            articulations[i] = articulation;
        }
    }
}

/// `notes[i]` の、前の列の単音からの音程の大きさ（半音）。どちらかの列が和音か先頭の列なら `None`。
pub fn slide_semitones(notes: &[Note], i: usize) -> Option<u8> {
    interval_from_previous(notes, i).map(|d| u8::try_from(d.unsigned_abs()).unwrap_or(u8::MAX))
}

fn interval_from_previous(notes: &[Note], i: usize) -> Option<i16> {
    let note = &notes[i];
    if note.column == 0 {
        return None;
    }
    let current = single_in_column(notes, note.column)?;
    let previous = single_in_column(notes, note.column - 1)?;
    Some(i16::from(current.pitch) - i16::from(previous.pitch))
}

fn slide(pitch: u8, d: i16) -> Option<Articulation> {
    let width = u8::try_from(d.unsigned_abs()).ok()?;
    if !(1..=SLIDE_MAX_SEMITONES).contains(&width) {
        return None;
    }
    if d > 0 {
        (SLIDE_UP_LOWEST_START + width..=SLIDE_HIGHEST_PITCH)
            .contains(&pitch)
            .then_some(Articulation::SlideUp)
    } else {
        (SLIDE_LOWEST_PITCH..=SLIDE_HIGHEST_PITCH - width)
            .contains(&pitch)
            .then_some(Articulation::SlideDown)
    }
}

fn bend(pitch: u8, d: i16) -> Option<Articulation> {
    let articulation = match d {
        1 => Articulation::BendHalf,
        2 => Articulation::BendWhole,
        3 => Articulation::BendWholeHalf,
        _ => return None,
    };
    BEND_PITCHES.contains(&pitch).then_some(articulation)
}

#[cfg(test)]
mod tests;
