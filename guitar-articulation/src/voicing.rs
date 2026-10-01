use std::ops::RangeInclusive;

use crate::{slide_semitones, Articulation, Note, Rule, RuleTable, SLIDE_MAX_SEMITONES};

/// パームミュート（`Mute_Down` / `Mute_Up`）の sample が在る音高。
pub const MUTE_PITCHES: RangeInclusive<u8> = 30..=76;

/// ピッキングハーモニクス（`PH`）の sample が在る音高。
pub const PINCH_HARMONIC_PITCHES: RangeInclusive<u8> = 30..=76;

/// ナチュラルハーモニクス（`Natural_Harmonics`）の sample が在る音高。
pub const NATURAL_HARMONICS_PITCHES: RangeInclusive<u8> = 35..=88;

/// ブラッシング（`Brush_Down` / `Brush_Up`）の sample が在る音高。
pub const BRUSH_PITCHES: RangeInclusive<u8> = 31..=51;

/// フレットミュート（`Mute_Fret_Down` / `Mute_Fret_Up`）の sample が在る音高。
pub const FRET_MUTE_PITCHES: RangeInclusive<u8> = 31..=88;

/// スライドアウト（`Slide_Out`）の sample が在る音高。
pub const SLIDE_OUT_PITCHES: RangeInclusive<u8> = 32..=88;

/// 擬似レガート（`Pseudo_Legato`）とポルタメント（`Portament`）の sample が在る音高。
pub const GLIDE_IN_PITCHES: RangeInclusive<u8> = 30..=88;

/// トリル（`Trill_HT` / `_WT` / `_min3` / `_Maj3`）の sample が在る音高。
pub const TRILL_PITCHES: RangeInclusive<u8> = 30..=88;

/// ユニゾンチョーキング（`Unison_Bend_Auto`）の sample が在る音高。
pub const UNISON_BEND_PITCHES: RangeInclusive<u8> = 60..=84;

/// スライドイン（`Slide_In`）の sample が在る最高音。最低音は幅で違う（[`slide_in_pitches`]）。
const SLIDE_IN_HIGHEST_PITCH: u8 = 88;

/// スライドインの幅（半音、1〜[`SLIDE_MAX_SEMITONES`]）。`semitones` は前の列の単音からの音程の大きさ
/// （[`slide_semitones`]）で、前の列が無いか和音なら 1。
pub fn slide_in_width(semitones: Option<u8>) -> u8 {
    semitones.unwrap_or(1).clamp(1, SLIDE_MAX_SEMITONES)
}

/// 幅 `width` のスライドインの sample が在る音高。幅 1〜6 は 32〜37 から、幅 7 は sfz の区切りどおり 32 から。
pub fn slide_in_pitches(width: u8) -> RangeInclusive<u8> {
    let lowest = match width {
        SLIDE_MAX_SEMITONES => 32,
        width => 31 + width,
    };
    lowest..=SLIDE_IN_HIGHEST_PITCH
}

/// 列ごとの「どう鳴らすか」のルールで、ピッキングする音（`Sus_Down` / `Sus_Up`）の奏法を写し替える。
///
/// - [`Rule::PalmMute`] / [`Rule::Brushing`] / [`Rule::FretMute`]: ストロークを保って写す。
/// - [`Rule::PinchHarmonic`] / [`Rule::NaturalHarmonics`] / [`Rule::SlideOut`]: ストロークに関わらず 1 つの奏法にする。
/// - [`Rule::PseudoLegato`] / [`Rule::Portamento`]: ピッキングしない音（H/P）も写す。
/// - トリル 4 種 / [`Rule::UnisonBendAuto`]: ストロークに関わらず 1 つの奏法にする。
/// - [`Rule::SlideIn`] は、音域が前の列からの幅（[`slide_in_width`]）で決まる。
/// - [`Rule::UnisonBendManual`] は、列の全部の音が音域に在り、他の列の音と重ならない列だけ写す。
///
/// sample の無い音高の音と、上に挙げた以外の奏法の音はそのまま残す。
pub fn apply_voicing_rules(notes: &[Note], rules: &RuleTable, articulations: &mut [Articulation]) {
    for (note, articulation) in notes.iter().zip(articulations.iter_mut()) {
        if let Some(voiced) = VOICINGS
            .iter()
            .filter(|voicing| rules.is_on(note.column, voicing.rule))
            .find_map(|voicing| voicing.voice(note.pitch, *articulation))
        {
            *articulation = voiced;
        }
    }
    for (i, note) in notes.iter().enumerate() {
        let width = slide_in_width(slide_semitones(notes, i));
        if rules.is_on(note.column, Rule::SlideIn)
            && matches!(
                articulations[i],
                Articulation::SusDown | Articulation::SusUp
            )
            && slide_in_pitches(width).contains(&note.pitch)
        {
            articulations[i] = Articulation::SlideIn;
        }
    }
    crate::unison_bend::apply_unison_bend_manual(notes, rules, articulations);
}

/// 1 ルールの写し方。
struct Voicing {
    rule: Rule,
    pitches: RangeInclusive<u8>,
    down: Articulation,
    up: Articulation,
    /// H/P の音も写すか。
    legato: bool,
}

impl Voicing {
    fn voice(&self, pitch: u8, articulation: Articulation) -> Option<Articulation> {
        if !self.pitches.contains(&pitch) {
            return None;
        }
        match articulation {
            Articulation::SusDown => Some(self.down),
            Articulation::SusUp => Some(self.up),
            Articulation::HammerOn | Articulation::PullOff if self.legato => Some(self.down),
            _ => None,
        }
    }
}

const VOICINGS: [Voicing; 13] = [
    Voicing {
        rule: Rule::PalmMute,
        pitches: MUTE_PITCHES,
        down: Articulation::MuteDown,
        up: Articulation::MuteUp,
        legato: false,
    },
    Voicing {
        rule: Rule::PinchHarmonic,
        pitches: PINCH_HARMONIC_PITCHES,
        down: Articulation::PinchHarmonic,
        up: Articulation::PinchHarmonic,
        legato: false,
    },
    Voicing {
        rule: Rule::NaturalHarmonics,
        pitches: NATURAL_HARMONICS_PITCHES,
        down: Articulation::NaturalHarmonics,
        up: Articulation::NaturalHarmonics,
        legato: false,
    },
    Voicing {
        rule: Rule::Brushing,
        pitches: BRUSH_PITCHES,
        down: Articulation::BrushDown,
        up: Articulation::BrushUp,
        legato: false,
    },
    Voicing {
        rule: Rule::FretMute,
        pitches: FRET_MUTE_PITCHES,
        down: Articulation::MuteFretDown,
        up: Articulation::MuteFretUp,
        legato: false,
    },
    Voicing {
        rule: Rule::SlideOut,
        pitches: SLIDE_OUT_PITCHES,
        down: Articulation::SlideOut,
        up: Articulation::SlideOut,
        legato: false,
    },
    Voicing {
        rule: Rule::PseudoLegato,
        pitches: GLIDE_IN_PITCHES,
        down: Articulation::PseudoLegato,
        up: Articulation::PseudoLegato,
        legato: true,
    },
    Voicing {
        rule: Rule::Portamento,
        pitches: GLIDE_IN_PITCHES,
        down: Articulation::Portamento,
        up: Articulation::Portamento,
        legato: true,
    },
    Voicing {
        rule: Rule::TrillHalf,
        pitches: TRILL_PITCHES,
        down: Articulation::TrillHalf,
        up: Articulation::TrillHalf,
        legato: false,
    },
    Voicing {
        rule: Rule::TrillWhole,
        pitches: TRILL_PITCHES,
        down: Articulation::TrillWhole,
        up: Articulation::TrillWhole,
        legato: false,
    },
    Voicing {
        rule: Rule::TrillMinorThird,
        pitches: TRILL_PITCHES,
        down: Articulation::TrillMinorThird,
        up: Articulation::TrillMinorThird,
        legato: false,
    },
    Voicing {
        rule: Rule::TrillMajorThird,
        pitches: TRILL_PITCHES,
        down: Articulation::TrillMajorThird,
        up: Articulation::TrillMajorThird,
        legato: false,
    },
    Voicing {
        rule: Rule::UnisonBendAuto,
        pitches: UNISON_BEND_PITCHES,
        down: Articulation::UnisonBendAuto,
        up: Articulation::UnisonBendAuto,
        legato: false,
    },
];

#[cfg(test)]
mod tests;
