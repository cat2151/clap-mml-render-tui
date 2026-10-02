//! 素材の音から、アルペジオの演奏を作る。
//!
//! 素材（MML 欄の演奏）は書き換えず、鳴らす列だけを作り直す。声部は素材の全音の音高を
//! 低い順・重複なしで並べたもので、オクターブレンジぶん +12 ずつ複製してから音型を当てる。
//! 1 step の秒は素材の最初の列から次の列までの間隔で、各音は 1 step ちょうど鳴らす。

use std::ops::RangeInclusive;

use cmrt_arpeggiator::{up_turn_sequence, ArpPattern, UP_TURN_DEFAULT};
use serde::{Deserialize, Serialize};

use crate::{notes_from_events, Note, TimedMidiEvent};

/// オクターブレンジ（声部集合を何オクターブぶん並べるか）の範囲。
pub const ARP_OCTAVES: RangeInclusive<usize> = 1..=3;
/// 音型の 1 周期を何回繰り返すかの範囲。
pub const ARP_CYCLES: RangeInclusive<usize> = 1..=8;
/// [`ArpPattern::UpTurn`] の戻り幅の範囲。
pub const ARP_TURN: RangeInclusive<usize> = 1..=3;

/// この画面で選べる音型。声部の並べ替え（Converge・Diverge）・最高音との交互（Octave）・
/// Random は持たない。
pub const ARP_PATTERNS: [ArpPattern; 6] = [
    ArpPattern::Up,
    ArpPattern::Down,
    ArpPattern::UpDown,
    ArpPattern::DownUp,
    ArpPattern::UpDownHold,
    ArpPattern::UpTurn,
];

/// アルペジエーターの設定。`enabled` が false の間も他の値は保つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArpSettings {
    pub enabled: bool,
    /// [`ArpPattern::label`] の綴りで読み書きする。[`ARP_PATTERNS`] に無い綴りは [`ArpPattern::Up`]。
    #[serde(with = "pattern_label")]
    pub pattern: ArpPattern,
    /// [`ARP_OCTAVES`] の値。
    pub octaves: usize,
    /// 音型の 1 周期の回数（[`ARP_CYCLES`]）。
    pub cycles: usize,
    /// [`ArpPattern::UpTurn`] の戻り幅（[`ARP_TURN`]）。
    pub turn: usize,
}

impl Default for ArpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            pattern: ArpPattern::Up,
            octaves: 1,
            cycles: 2,
            turn: UP_TURN_DEFAULT,
        }
    }
}

mod pattern_label {
    use cmrt_arpeggiator::ArpPattern;
    use serde::{Deserialize, Deserializer, Serializer};

    use super::ARP_PATTERNS;

    pub fn serialize<S: Serializer>(
        pattern: &ArpPattern,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(pattern.label())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ArpPattern, D::Error> {
        let label = String::deserialize(deserializer)?;
        Ok(ARP_PATTERNS
            .into_iter()
            .find(|pattern| pattern.label() == label)
            .unwrap_or(ArpPattern::Up))
    }
}

/// MML 欄の素材から、鳴らすイベント列を作る。空の MML は空、それ以外は
/// [`cmrt_chord::timed_performance`] に [`arpeggiate`] を当てたもの。
pub fn performance_events(mml: &str, arp: &ArpSettings) -> Result<Vec<TimedMidiEvent>, String> {
    if mml.is_empty() {
        return Ok(Vec::new());
    }
    let plain = cmrt_chord::timed_performance(mml)?.events;
    Ok(arpeggiate(&plain, arp))
}

/// 素材の音を声部にしたアルペジオの note on / note off を返す。`enabled` が false なら素材のまま。
///
/// 長さは音型の 1 周期 × `cycles`。各音の velocity と channel は、素材で同じ音高を最初に鳴らした音のもの。
/// 範囲外の設定値は範囲の端へ寄せる。素材に音が無いか、刻みが 0 なら素材のまま。
pub fn arpeggiate(plain: &[TimedMidiEvent], settings: &ArpSettings) -> Vec<TimedMidiEvent> {
    if !settings.enabled {
        return plain.to_vec();
    }
    let notes = notes_from_events(plain);
    let Some(step) = step_seconds(&notes).filter(|step| *step > 0.0) else {
        return plain.to_vec();
    };
    let voices = voices(&notes, clamp(settings.octaves, ARP_OCTAVES));
    let sequence = voice_order(settings, voices.len());
    let start = notes[0].on_seconds;
    let mut out: Vec<TimedMidiEvent> = sequence
        .iter()
        .enumerate()
        .flat_map(|(i, &voice)| {
            let note = &voices[voice];
            let on = start + i as f64 * step;
            [
                TimedMidiEvent {
                    seconds: on,
                    message: [0x90 | note.channel, note.pitch, note.velocity],
                },
                TimedMidiEvent {
                    seconds: on + step,
                    message: [0x80 | note.channel, note.pitch, 0],
                },
            ]
        })
        .collect();
    cmrt_midi_filter::sort_for_playback(&mut out);
    out
}

fn clamp(value: usize, range: RangeInclusive<usize>) -> usize {
    value.clamp(*range.start(), *range.end())
}

/// 最初の列の note on から次の列の note on までの秒。列が 1 つなら最初の音の長さ。
fn step_seconds(notes: &[Note]) -> Option<f64> {
    let first = notes.first()?;
    Some(
        match notes.iter().find(|note| note.column != first.column) {
            Some(next) => next.on_seconds - first.on_seconds,
            None => first.off_seconds - first.on_seconds,
        },
    )
}

/// 低い順・重複なしの音高を、`octaves` ぶん +12 ずつ複製した声部。MIDI の音域を超える音は落とす。
fn voices(notes: &[Note], octaves: usize) -> Vec<Note> {
    let mut base: Vec<Note> = Vec::new();
    for note in notes {
        if !base.iter().any(|voice| voice.pitch == note.pitch) {
            base.push(*note);
        }
    }
    base.sort_by_key(|note| note.pitch);
    (0..octaves as u8)
        .flat_map(|octave| {
            base.iter().filter_map(move |note| {
                let pitch = note.pitch.checked_add(octave * 12).filter(|p| *p <= 127)?;
                Some(Note { pitch, ..*note })
            })
        })
        .collect()
}

/// 音型の 1 周期を `cycles` 回並べた声部番号。
fn voice_order(settings: &ArpSettings, voice_count: usize) -> Vec<usize> {
    let cycles = clamp(settings.cycles, ARP_CYCLES);
    let period = match settings.pattern {
        ArpPattern::UpTurn => up_turn_sequence(voice_count, clamp(settings.turn, ARP_TURN)),
        pattern => pattern.voice_sequence(voice_count).unwrap_or_default(),
    };
    period.repeat(cycles)
}

#[cfg(test)]
mod tests;
