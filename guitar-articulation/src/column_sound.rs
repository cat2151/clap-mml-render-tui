//! 列のいちばん低い音 1 つだけを、奏法の sample が在る音高へ差し替えて鳴らす列ルール
//! （ピックスクレイプ・クロマチックラン・スライドエフェクト・効果音）。

use std::ops::RangeInclusive;

use crate::notes::{is_note_off, is_note_on};
use crate::{pick_scratch_pitch, Articulated, Articulation, Note, Rule, RuleTable, TimedMidiEvent};

/// クロマチックラン（`Chromatic_Run`）の sample が在る音高。1 キー = 1 フレーズで、
/// 30〜32 がミュート、33〜35 がサステイン、36〜38 がフレット（それぞれ F#1 / B1 / E2 始まり）。
pub const CHROMATIC_RUN_PITCHES: RangeInclusive<u8> = 30..=38;

/// クロマチックランで音高を畳む 1 オクターブ。畳んだ音高が [`CHROMATIC_RUN_PITCHES`] の外なら効かない。
const CHROMATIC_RUN_FOLD: RangeInclusive<u8> = 30..=41;

/// 滑り下りるスライドエフェクト（`Slide_FX_Down`）の sample が在る音高。
pub const SLIDE_FX_DOWN_PITCHES: RangeInclusive<u8> = 42..=66;

/// 滑り上がる / 上下に滑るスライドエフェクト（`Slide_FX_Up` / `Slide_FX_Wow`）の sample が在る音高。
pub const SLIDE_FX_UP_PITCHES: RangeInclusive<u8> = 30..=54;

/// 効果音の note number。KS の範囲（E-1 = 4 から）より下なので、KS のラッチを変えずに鳴る。
pub const EFFECT_HELLO_PITCH: u8 = 0;
pub const EFFECT_RESONANCE_PITCH: u8 = 1;
pub const EFFECT_SLIDE_NOISE_PITCH: u8 = 2;
pub const EFFECT_HARD_STOP_PITCH: u8 = 3;

/// `pitch` を、オクターブ単位で `range` へ畳んだ音高。`range` は 12 半音以上の幅を前提にする。
pub fn fold_pitch(pitch: u8, range: RangeInclusive<u8>) -> u8 {
    let (low, high) = (*range.start(), *range.end());
    let mut folded = pitch;
    while folded > high {
        folded -= 12;
    }
    while folded < low {
        folded += 12;
    }
    folded
}

/// 1 ルールの鳴らし方。
struct ColumnSound {
    rule: Rule,
    /// 列の音の奏法（KS）。`None` は KS の要らない効果音で、奏法を変えない。
    articulation: Option<Articulation>,
    /// 列のいちばん低い音の音高から、鳴らす音高。`None` はその列に効かない。
    pitch: fn(u8) -> Option<u8>,
}

const COLUMN_SOUNDS: [ColumnSound; 9] = [
    ColumnSound {
        rule: Rule::PickScratch,
        articulation: Some(Articulation::PickScratch),
        pitch: pick_scratch,
    },
    ColumnSound {
        rule: Rule::ChromaticRun,
        articulation: Some(Articulation::ChromaticRun),
        pitch: chromatic_run,
    },
    ColumnSound {
        rule: Rule::SlideFxDown,
        articulation: Some(Articulation::SlideFxDown),
        pitch: slide_fx_down,
    },
    ColumnSound {
        rule: Rule::SlideFxUp,
        articulation: Some(Articulation::SlideFxUp),
        pitch: slide_fx_up,
    },
    ColumnSound {
        rule: Rule::SlideFxWow,
        articulation: Some(Articulation::SlideFxWow),
        pitch: slide_fx_up,
    },
    ColumnSound {
        rule: Rule::EffectHello,
        articulation: None,
        pitch: |_| Some(EFFECT_HELLO_PITCH),
    },
    ColumnSound {
        rule: Rule::EffectResonance,
        articulation: None,
        pitch: |_| Some(EFFECT_RESONANCE_PITCH),
    },
    ColumnSound {
        rule: Rule::EffectSlideNoise,
        articulation: None,
        pitch: |_| Some(EFFECT_SLIDE_NOISE_PITCH),
    },
    ColumnSound {
        rule: Rule::EffectHardStop,
        articulation: None,
        pitch: |_| Some(EFFECT_HARD_STOP_PITCH),
    },
];

fn pick_scratch(pitch: u8) -> Option<u8> {
    Some(pick_scratch_pitch(pitch))
}

fn chromatic_run(pitch: u8) -> Option<u8> {
    Some(fold_pitch(pitch, CHROMATIC_RUN_FOLD)).filter(|key| CHROMATIC_RUN_PITCHES.contains(key))
}

fn slide_fx_down(pitch: u8) -> Option<u8> {
    Some(fold_pitch(pitch, SLIDE_FX_DOWN_PITCHES))
}

fn slide_fx_up(pitch: u8) -> Option<u8> {
    Some(fold_pitch(pitch, SLIDE_FX_UP_PITCHES))
}

/// 上のルールが ON の列の奏法を上書きし（それまでの奏法を問わない）、音ごとの鳴らす音高を返す。
/// `None` はその音を鳴らさない（列のいちばん低い音以外）。
///
/// 効かない列（クロマチックランで畳んだ音高に sample が無い）とルールの無い列は、奏法も音高も元のまま。
/// `notes` は列順を前提にする。
pub(crate) fn apply_column_sounds(
    notes: &[Note],
    rules: &RuleTable,
    articulations: &mut [Articulation],
) -> Vec<Option<u8>> {
    let mut pitches: Vec<Option<u8>> = notes.iter().map(|note| Some(note.pitch)).collect();
    let mut start = 0;
    for chunk in notes.chunk_by(|a, b| a.column == b.column) {
        let range = start..start + chunk.len();
        start = range.end;
        let column = chunk[0].column;
        let Some(sound) = COLUMN_SOUNDS
            .iter()
            .find(|sound| rules.is_on(column, sound.rule))
        else {
            continue;
        };
        let lowest = range
            .clone()
            .min_by_key(|&i| notes[i].pitch)
            .expect("列には音が 1 つ以上ある");
        let Some(pitch) = (sound.pitch)(notes[lowest].pitch) else {
            continue;
        };
        for i in range {
            if let Some(articulation) = sound.articulation {
                articulations[i] = articulation;
            }
            pitches[i] = (i == lowest).then_some(pitch);
        }
    }
    pitches
}

/// raw の note on/off を、[`Articulated::pitch`] の音高へ差し替える。鳴らさない音の note on/off は `None`。
/// note 以外のイベントと、どの音にも当たらないイベントはそのまま返す。
pub(crate) fn sounding_event(
    event: &TimedMidiEvent,
    notes: &[Note],
    articulated: &[Articulated],
) -> Option<TimedMidiEvent> {
    let [status, pitch, _] = event.message;
    let on = is_note_on(&event.message);
    if !on && !is_note_off(&event.message) {
        return Some(*event);
    }
    let found = notes.iter().zip(articulated).find(|(note, _)| {
        let seconds = if on {
            note.on_seconds
        } else {
            note.off_seconds
        };
        note.channel == status & 0x0F
            && note.pitch == pitch
            && (seconds - event.seconds).abs() < 1e-9
    });
    let Some((_, a)) = found else {
        return Some(*event);
    };
    a.pitch.map(|sounding| {
        let mut event = *event;
        event.message[1] = sounding;
        event
    })
}

#[cfg(test)]
mod tests;
