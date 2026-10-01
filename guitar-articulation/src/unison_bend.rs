//! ユニゾンチョーキング（手動、`Unison_Bend_Manual`）の列と、その列に足す pitch bend。
//!
//! 手動版は 2 層が重なって鳴り、片方（2 半音下の sample）だけが pitch bend で持ち上がる
//! （sfz の `bend_up=215` / `bend_up=15`）。pitch bend を最大まで上げると 2 層が同じ音程に揃う。
//! 曲げ方は付属のサンプル MID（`METAL-GTX_Demo2.MID`）に倣い、頭から直線で上げ、保ち、音の終わりで中央へ戻す。

use std::collections::BTreeSet;

use crate::{Articulation, Note, Rule, RuleTable, TimedMidiEvent, UNISON_BEND_PITCHES};

/// 上げきるまでの長さの上限。
pub const UNISON_BEND_RISE_SECONDS: f64 = 0.35;

/// 中央へ戻す長さの上限。音の終わりで中央に着く。
pub const UNISON_BEND_FALL_SECONDS: f64 = 0.13;

/// 音が短いとき、上げる区間と戻す区間が音の長さに占める割合の上限。
const RISE_SHARE: f64 = 0.6;
const FALL_SHARE: f64 = 0.2;

/// pitch bend を送る間隔（サンプル MID の刻みと同じ程度）。
pub const UNISON_BEND_STEP_SECONDS: f64 = 0.0175;

/// pitch bend の中央と最大（14 bit）。
const CENTER: u16 = 8192;
const TOP: u16 = 16383;

/// [`Rule::UnisonBendManual`] が効く列。
///
/// pitch bend は channel 全体に効くので、列の音が鳴っている間に他の列の音が重なるなら効かない。
/// 列の全部の音がピッキングする音（`Sus_Down` / `Sus_Up`）で、音域（[`UNISON_BEND_PITCHES`]）に在ることも要る
/// （音域外の音は通常の sample のまま曲がってしまう）。
fn manual_columns(
    notes: &[Note],
    rules: &RuleTable,
    articulations: &[Articulation],
) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for (note, _) in notes.iter().zip(articulations) {
        let column = note.column;
        if out.contains(&column) || !rules.is_on(column, Rule::UnisonBendManual) {
            continue;
        }
        let members = || {
            notes
                .iter()
                .zip(articulations)
                .filter(move |(n, _)| n.column == column)
        };
        let voiceable = members().all(|(n, a)| {
            UNISON_BEND_PITCHES.contains(&n.pitch)
                && matches!(a, Articulation::SusDown | Articulation::SusUp)
        });
        let on = members()
            .map(|(n, _)| n.on_seconds)
            .fold(f64::MAX, f64::min);
        let off = members().map(|(n, _)| n.off_seconds).fold(0.0, f64::max);
        let overlapped = notes
            .iter()
            .any(|n| n.column != column && n.on_seconds < off && n.off_seconds > on);
        if voiceable && !overlapped {
            out.insert(column);
        }
    }
    out
}

/// 効く列（[`manual_columns`]）の音を `Unison_Bend_Manual` にする。
pub(crate) fn apply_unison_bend_manual(
    notes: &[Note],
    rules: &RuleTable,
    articulations: &mut [Articulation],
) {
    let columns = manual_columns(notes, rules, articulations);
    for (note, articulation) in notes.iter().zip(articulations.iter_mut()) {
        if columns.contains(&note.column) {
            *articulation = Articulation::UnisonBendManual;
        }
    }
}

/// `Unison_Bend_Manual` の音が在る列ごとに、列の頭から pitch bend を上げ、保ち、列の最後の note off で中央へ戻す。
/// 1 つでも送ったら `end` でも中央へ戻す（pitch bend は演奏を跨いで残る）。
pub(crate) fn pitch_bend_events(
    notes: &[Note],
    articulations: &[Articulation],
    end: f64,
) -> Vec<TimedMidiEvent> {
    let columns: BTreeSet<usize> = notes
        .iter()
        .zip(articulations)
        .filter(|(_, a)| **a == Articulation::UnisonBendManual)
        .map(|(n, _)| n.column)
        .collect();
    let mut out = Vec::new();
    let mut channel = 0;
    for column in columns {
        let members = || notes.iter().filter(|n| n.column == column);
        channel = members().next().map_or(0, |n| n.channel);
        let on = members().map(|n| n.on_seconds).fold(f64::MAX, f64::min);
        let off = members().map(|n| n.off_seconds).fold(0.0, f64::max);
        out.extend(bend_curve(on, off, channel));
    }
    if !out.is_empty() {
        out.push(pitch_bend(end, channel, CENTER));
    }
    out
}

/// `on` 〜 `off` の 1 音の曲げ方。上げる区間と戻す区間は、それぞれ端の値を含めて刻む。
fn bend_curve(on: f64, off: f64, channel: u8) -> Vec<TimedMidiEvent> {
    let length = (off - on).max(0.0);
    let rise = UNISON_BEND_RISE_SECONDS.min(length * RISE_SHARE);
    let fall = UNISON_BEND_FALL_SECONDS.min(length * FALL_SHARE);
    let mut out = ramp(on, rise, CENTER, TOP, channel);
    out.extend(ramp(off - fall, fall, TOP, CENTER, channel));
    out
}

/// `start` から `seconds` かけて `from` から `to` へ直線で動かす。
fn ramp(start: f64, seconds: f64, from: u16, to: u16, channel: u8) -> Vec<TimedMidiEvent> {
    let steps = (seconds / UNISON_BEND_STEP_SECONDS).round().max(1.0) as u32;
    (0..=steps)
        .map(|k| {
            let share = f64::from(k) / f64::from(steps);
            let value = f64::from(from) + (f64::from(to) - f64::from(from)) * share;
            pitch_bend(start + seconds * share, channel, value.round() as u16)
        })
        .collect()
}

fn pitch_bend(seconds: f64, channel: u8, value: u16) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0xE0 | channel, (value & 0x7F) as u8, (value >> 7) as u8],
    }
}

#[cfg(test)]
mod tests;
