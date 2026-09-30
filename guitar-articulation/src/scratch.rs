use std::ops::RangeInclusive;

use crate::notes::{is_note_off, is_note_on};
use crate::{Articulated, Articulation, Note, Rule, RuleTable, TimedMidiEvent};

/// ピックスクレイプ（`Pick_Scratch`）の sample が在る音高。30 が原音で、上ほど速く明るく擦る。
pub const PICK_SCRATCH_PITCHES: RangeInclusive<u8> = 30..=42;

/// [`Rule::PickScratch`] の列の音を、全部 `Pick_Scratch` にする（それまでの奏法を上書きする）。
pub fn apply_pick_scratch(notes: &[Note], rules: &RuleTable, articulations: &mut [Articulation]) {
    for (note, articulation) in notes.iter().zip(articulations.iter_mut()) {
        if rules.is_on(note.column, Rule::PickScratch) {
            *articulation = Articulation::PickScratch;
        }
    }
}

/// MML の音高を、オクターブ単位で [`PICK_SCRATCH_PITCHES`] へ畳んだ音高。
pub fn pick_scratch_pitch(pitch: u8) -> u8 {
    let (low, high) = (*PICK_SCRATCH_PITCHES.start(), *PICK_SCRATCH_PITCHES.end());
    let mut folded = pitch;
    while folded > high {
        folded -= 12;
    }
    while folded < low {
        folded += 12;
    }
    folded
}

/// 音ごとに鳴らす音高。`None` はその音を鳴らさない。
///
/// `Pick_Scratch` の音は、列のいちばん低い音 1 つだけを [`pick_scratch_pitch`] で鳴らす
/// （擦る音は和音にならない）。それ以外の音は MML の音高のまま。
pub(crate) fn sounding_pitches(notes: &[Note], articulations: &[Articulation]) -> Vec<Option<u8>> {
    notes
        .iter()
        .zip(articulations)
        .enumerate()
        .map(|(i, (note, &articulation))| {
            if articulation != Articulation::PickScratch {
                return Some(note.pitch);
            }
            let lowest = notes
                .iter()
                .enumerate()
                .filter(|(_, other)| other.column == note.column)
                .min_by_key(|(_, other)| other.pitch)
                .map(|(index, _)| index);
            (lowest == Some(i)).then(|| pick_scratch_pitch(note.pitch))
        })
        .collect()
}

/// raw の note on/off を、[`Articulated::pitch`] の音高へ差し替える。鳴らさない音の note on/off は `None`。
/// `Pick_Scratch` 以外の音と、note 以外のイベントはそのまま返す。
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
    let scratch = notes.iter().zip(articulated).find(|(note, a)| {
        let seconds = if on {
            note.on_seconds
        } else {
            note.off_seconds
        };
        a.articulation == Articulation::PickScratch
            && note.channel == status & 0x0F
            && note.pitch == pitch
            && (seconds - event.seconds).abs() < 1e-9
    });
    let Some((_, a)) = scratch else {
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
