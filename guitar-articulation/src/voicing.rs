use std::ops::RangeInclusive;

use crate::{Articulation, Note, Rule, RuleTable};

/// パームミュート（`Mute_Down` / `Mute_Up`）の sample が在る音高。
pub const MUTE_PITCHES: RangeInclusive<u8> = 30..=76;

/// ピッキングハーモニクス（`PH`）の sample が在る音高。
pub const PINCH_HARMONIC_PITCHES: RangeInclusive<u8> = 30..=76;

/// 列ごとの「どう鳴らすか」のルールで、ピッキングする音（`Sus_Down` / `Sus_Up`）の奏法を写し替える。
///
/// - [`Rule::PalmMute`]: ストロークを保って `Mute_Down` / `Mute_Up` にする。
/// - [`Rule::PinchHarmonic`]: ストロークに関わらず `PH` にする。
///
/// sample の無い音高の音と、ピッキングしない音（H/P）はそのまま残す。
pub fn apply_voicing_rules(notes: &[Note], rules: &RuleTable, articulations: &mut [Articulation]) {
    for (note, articulation) in notes.iter().zip(articulations.iter_mut()) {
        if rules.is_on(note.column, Rule::PalmMute) && MUTE_PITCHES.contains(&note.pitch) {
            *articulation = match *articulation {
                Articulation::SusDown => Articulation::MuteDown,
                Articulation::SusUp => Articulation::MuteUp,
                other => other,
            };
        } else if rules.is_on(note.column, Rule::PinchHarmonic)
            && PINCH_HARMONIC_PITCHES.contains(&note.pitch)
            && matches!(*articulation, Articulation::SusDown | Articulation::SusUp)
        {
            *articulation = Articulation::PinchHarmonic;
        }
    }
}

#[cfg(test)]
mod tests;
