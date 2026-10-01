use std::ops::RangeInclusive;

use rand::{rngs::StdRng, Rng, RngExt, SeedableRng};

use crate::notes::{is_note_off, is_note_on};
use crate::{
    control_events, keyswitch_events, Articulated, Articulation, Note, RuleTable, TimedMidiEvent,
};

/// 汚しの乱数の seed。固定なので、同じ MML とルール表からは同じ演奏を作り直せる。
pub(crate) const HUMANIZE_SEED: u64 = 0x6775_6974_6172;

/// note on をずらす幅（±秒）。
pub(crate) const HUMANIZE_ON_SECONDS: f64 = 0.008;
/// note off をずらす幅（±秒）。off はアタックより聞こえにくいので on より広い。
pub(crate) const HUMANIZE_OFF_SECONDS: f64 = 0.015;
/// アクセントでない音の velocity を散らす幅（±）。
pub(crate) const VELOCITY_SPREAD: i32 = 8;
/// アクセントの velocity を元の値から下げる最大量（上限の 127 から上へは散らせない）。
pub(crate) const ACCENT_VELOCITY_DROP: i32 = 8;
/// ピッキングノイズ層の音量（CC30）。sfz の既定 75 の周り。
pub(crate) const PICKING_CC30_RANGE: RangeInclusive<u8> = 50..=100;
/// ピッキングノイズ層の音量を決める CC（METAL-GTX の `Picking`）。
pub(crate) const PICKING_CC: u8 = 30;
/// sfz の `set_cc30`。演奏の終わりにこの値へ戻す。
pub(crate) const PICKING_CC_DEFAULT: u8 = 75;
/// on のずれを、隣の列の on との間隔の何割までに抑えるか。0.5 未満なら隣の列を追い越さない。
pub(crate) const NEIGHBOR_GAP_FRACTION: f64 = 0.4;
/// off をずらした後も残す、最短の音長（秒）。
pub(crate) const MIN_NOTE_SECONDS: f64 = 0.020;

/// 汚しを掛けた 1 音。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Humanized {
    pub on_seconds: f64,
    pub off_seconds: f64,
    pub velocity: u8,
    /// ピッキングする音だけ、直前に送る CC30 の値。
    pub picking: Option<u8>,
}

/// 音ごとに on/off の時刻・velocity・ピッキングノイズの量をばらつかせる。
///
/// velocity は [`articulate`](crate::articulate) が決めた強弱を散らすだけで、割合は掛けない。
/// 乱数は音ごとに同じ順・同じ回数だけ引くので、抑えが効いても後ろの音の結果は変わらない。
pub(crate) fn humanize(
    notes: &[Note],
    articulated: &[Articulated],
    rng: &mut impl Rng,
) -> Vec<Humanized> {
    let column_on = column_on_seconds(notes);
    let all = articulated;
    let ons: Vec<f64> = notes
        .iter()
        .map(|note| {
            let limit = on_limit(&column_on, note.column);
            let shift = rng.random_range(-HUMANIZE_ON_SECONDS..=HUMANIZE_ON_SECONDS);
            (note.on_seconds + shift.clamp(-limit, limit)).max(0.0)
        })
        .collect();

    notes
        .iter()
        .zip(articulated)
        .enumerate()
        .map(|(i, (note, articulated))| {
            let off_shift = rng.random_range(-HUMANIZE_OFF_SECONDS..=HUMANIZE_OFF_SECONDS);
            let velocity = i32::from(articulated.velocity)
                + if articulated.accent {
                    -rng.random_range(0..=ACCENT_VELOCITY_DROP)
                } else {
                    rng.random_range(-VELOCITY_SPREAD..=VELOCITY_SPREAD)
                };
            let cc30 = rng.random_range(PICKING_CC30_RANGE);
            let picks = matches!(
                articulated.articulation,
                Articulation::SusDown | Articulation::SusUp
            );
            Humanized {
                on_seconds: ons[i],
                off_seconds: off_seconds(notes, all, &ons, i, note.off_seconds + off_shift),
                velocity: u8::try_from(velocity.clamp(1, 127)).expect("clamped to 1..=127"),
                picking: picks.then_some(cc30),
            }
        })
        .collect()
}

/// [`HUMANIZE_SEED`] で引いた [`humanize`]。演奏に使う汚しはいつもこれ。
pub(crate) fn seeded(notes: &[Note], articulated: &[Articulated]) -> Vec<Humanized> {
    humanize(
        notes,
        articulated,
        &mut StdRng::seed_from_u64(HUMANIZE_SEED),
    )
}

/// raw の列を、汚しを掛けた演奏の列にする（並べ替えは呼び出し側）。
///
/// 演奏音の note on/off は [`humanize`] の時刻・velocity と [`Articulated::pitch`] で作り直し、note 以外のイベントはそのまま残す。
/// KS と CC はずらした後の音から作るので、KS は列でいちばん早い note on に付いていく。
/// ピッキングする音の note on と同時刻に CC30 を置き、演奏の終わりで sfz の既定値へ戻す
/// （CC は演奏を跨いで残る）。同時刻では並べ替えで note off → CC → KS → 演奏音の順になる。
/// `humanized` は `notes` と同じ並び（[`humanize`] の出力）。
pub(crate) fn humanized_events(
    events: &[TimedMidiEvent],
    notes: &[Note],
    articulated: &[Articulated],
    humanized: &[Humanized],
    rules: &RuleTable,
) -> Vec<TimedMidiEvent> {
    let shifted = shifted_notes(notes, humanized);
    let articulations: Vec<Articulation> = articulated.iter().map(|a| a.articulation).collect();

    let mut out = keyswitch_events(&shifted, &articulations);
    out.extend(control_events(&shifted, &articulations, rules));
    out.extend(picking_noise_events(&shifted, humanized));
    out.extend(
        events
            .iter()
            .filter(|event| !is_note_on(&event.message) && !is_note_off(&event.message))
            .copied(),
    );
    for (note, pitch) in shifted.iter().zip(articulated.iter().map(|a| a.pitch)) {
        let Some(pitch) = pitch else {
            continue;
        };
        out.push(TimedMidiEvent {
            seconds: note.on_seconds,
            message: [0x90 | note.channel, pitch, note.velocity],
        });
        out.push(TimedMidiEvent {
            seconds: note.off_seconds,
            message: [0x80 | note.channel, pitch, 0],
        });
    }
    out
}

/// 汚しの時刻・velocity で作り直した音。`humanized` は `notes` と同じ並び（[`humanize`] の出力）。
pub(crate) fn shifted_notes(notes: &[Note], humanized: &[Humanized]) -> Vec<Note> {
    notes
        .iter()
        .zip(humanized)
        .map(|(note, h)| Note {
            on_seconds: h.on_seconds,
            off_seconds: h.off_seconds,
            velocity: h.velocity,
            ..*note
        })
        .collect()
}

fn picking_noise_events(notes: &[Note], humanized: &[Humanized]) -> Vec<TimedMidiEvent> {
    let Some(first) = notes.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (note, h) in notes.iter().zip(humanized) {
        if let Some(cc30) = h.picking {
            out.push(control_change(
                note.on_seconds,
                note.channel,
                PICKING_CC,
                cc30,
            ));
        }
    }
    let end = notes.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
    out.push(control_change(
        end,
        first.channel,
        PICKING_CC,
        PICKING_CC_DEFAULT,
    ));
    out
}

pub(crate) fn control_change(
    seconds: f64,
    channel: u8,
    controller: u8,
    value: u8,
) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0xB0 | channel, controller, value],
    }
}

/// 列番号 → その列の元の on 時刻。
fn column_on_seconds(notes: &[Note]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for note in notes {
        if out.len() == note.column {
            out.push(note.on_seconds);
        }
    }
    out
}

fn on_limit(column_on: &[f64], column: usize) -> f64 {
    let before = column
        .checked_sub(1)
        .map(|previous| column_on[column] - column_on[previous]);
    let after = column_on
        .get(column + 1)
        .map(|next| next - column_on[column]);
    [before, after]
        .into_iter()
        .flatten()
        .fold(HUMANIZE_ON_SECONDS, |limit, gap| {
            limit.min(gap * NEIGHBOR_GAP_FRACTION)
        })
}

/// 最短の音長を守り、同じ channel・鳴らす音高（[`Articulated::pitch`]）の次の on を越えない off
/// （越えると off が次の音を止める）。
fn off_seconds(
    notes: &[Note],
    articulated: &[Articulated],
    ons: &[f64],
    i: usize,
    wanted: f64,
) -> f64 {
    let (note, pitch) = (&notes[i], articulated[i].pitch);
    let off = wanted.max(ons[i] + MIN_NOTE_SECONDS);
    notes[i + 1..]
        .iter()
        .zip(&articulated[i + 1..])
        .zip(&ons[i + 1..])
        .find(|((next, a), _)| {
            next.column > note.column && next.channel == note.channel && a.pitch == pitch
        })
        .map_or(off, |(_, &next_on)| off.min(next_on))
}

#[cfg(test)]
mod tests;
