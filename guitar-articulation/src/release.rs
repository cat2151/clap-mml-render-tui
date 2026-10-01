//! 汚し（リリース）: 音を離したときに鳴るリリース層の種類と音量を、列ごとにばらつかせる。
//!
//! METAL-GTX のリリース層（`trigger=release`）は CC24 の帯で種類が、CC25 で音量が決まる。
//! 使う帯は 2〜63 の 4 つ:
//!
//! | CC24 | 種類 |
//! |---|---|
//! | 2〜15 | Basic |
//! | 16〜31 | Hard |
//! | 32〜47 | Agressive |
//! | 48〜63 | Agressive2 |
//!
//! 0〜1 はリリース音が鳴らないので使わない（ばらつきではなく消音になる）。64〜79 は手の
//! ポジション移動の音で、移動していない所で鳴らすと汚しにならない。80〜127 はアクションリリース
//! （自動スライドアウト / 自動オルタネイト）で、別の奏法になる。

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use rand::{rngs::StdRng, Rng, RngExt, SeedableRng};

use crate::humanize::control_change;
use crate::{Note, TimedMidiEvent};

/// 汚し（リリース）の乱数の seed。汚し（[`HUMANIZE_SEED`](crate::humanize::HUMANIZE_SEED)）とは別の乱数列にして、
/// 片方の ON/OFF がもう片方の揺れを変えないようにする。
pub(crate) const RELEASE_SEED: u64 = 0x0072_656c_6561_7365;

/// リリース層の種類を選ぶ CC（METAL-GTX の `Rel_Shape`）。
pub(crate) const RELEASE_SHAPE_CC: u8 = 24;
/// リリース層の音量を決める CC（METAL-GTX の `Rel_Level`）。音量はおよそ `-110 + 110 × 値 / 127` dB。
pub(crate) const RELEASE_LEVEL_CC: u8 = 25;
/// CC24 を引く範囲。Basic / Hard / Agressive / Agressive2 の 4 帯。
pub(crate) const RELEASE_SHAPE_RANGE: RangeInclusive<u8> = 2..=63;
/// CC25 を引く範囲。95 で約 -28 dB、127 で 0 dB。
pub(crate) const RELEASE_LEVEL_RANGE: RangeInclusive<u8> = 95..=127;
/// sfz の `set_cc24`。演奏の終わりにこの値へ戻す。
pub(crate) const RELEASE_SHAPE_DEFAULT: u8 = 13;
/// sfz の `set_cc25`。演奏の終わりにこの値へ戻す。
pub(crate) const RELEASE_LEVEL_DEFAULT: u8 = 108;

/// 列ごとに CC24 と CC25 を 1 組、その列でいちばん早い note on と同時刻に置く。
///
/// リリース層は note off で鳴るので、その音の note on の時点で送っておけば off の前に届く。
/// 同時刻の CC は最後の 1 組しか効かないので、和音の列も 1 組だけ。channel は列の最初の音。
/// CC は演奏を跨いで残るので、演奏の終わり（全音の off の最大）で sfz の既定値へ戻す。
/// `notes` は列の順（[`notes_from_events`](crate::notes_from_events) の並び）。
/// `positions` の列は CC25 だけ置き、CC24 は置かない。その列の CC24 は CC24 を選ぶ列のルール
/// （[`crate::Rule::selects_release_shape`]）が同じ時刻に送るので、置くと同時刻に CC24 が 2 つ並ぶ。
/// 乱数は引いてから捨てる（引き順を変えないので、他の列の値は変わらない）。
pub(crate) fn release_events(
    notes: &[Note],
    positions: &BTreeSet<usize>,
    rng: &mut impl Rng,
) -> Vec<TimedMidiEvent> {
    let Some(first) = notes.first() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut start = 0;
    while start < notes.len() {
        let column = notes[start].column;
        let len = notes[start..]
            .iter()
            .take_while(|note| note.column == column)
            .count();
        let members = &notes[start..start + len];
        let on = members
            .iter()
            .map(|note| note.on_seconds)
            .fold(f64::INFINITY, f64::min);
        let channel = members[0].channel;
        let shape = rng.random_range(RELEASE_SHAPE_RANGE);
        let level = rng.random_range(RELEASE_LEVEL_RANGE);
        if !positions.contains(&column) {
            out.push(control_change(on, channel, RELEASE_SHAPE_CC, shape));
        }
        out.push(control_change(on, channel, RELEASE_LEVEL_CC, level));
        start += len;
    }
    let end = notes.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
    out.push(control_change(
        end,
        first.channel,
        RELEASE_SHAPE_CC,
        RELEASE_SHAPE_DEFAULT,
    ));
    out.push(control_change(
        end,
        first.channel,
        RELEASE_LEVEL_CC,
        RELEASE_LEVEL_DEFAULT,
    ));
    out
}

/// CC24 の値が選ぶリリース層の種類の名前。
pub(crate) fn release_shape_name(value: u8) -> &'static str {
    match value {
        0..=1 => "none",
        2..=15 => "Basic",
        16..=31 => "Hard",
        32..=47 => "Agressive",
        48..=63 => "Agressive2",
        64..=79 => "position",
        80..=95 => "slide out",
        // Sus_Down / Mute_Down などはアップストローク、他はスライドアウト。
        _ => "alternate",
    }
}

/// [`RELEASE_SEED`] で引いた [`release_events`]。演奏に使う汚し（リリース）はいつもこれ。
pub(crate) fn seeded_release_events(
    notes: &[Note],
    positions: &BTreeSet<usize>,
) -> Vec<TimedMidiEvent> {
    release_events(notes, positions, &mut StdRng::seed_from_u64(RELEASE_SEED))
}

#[cfg(test)]
mod tests;
