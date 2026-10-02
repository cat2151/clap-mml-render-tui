//! 素材の音から、アルペジオの演奏を作る。
//!
//! 素材（MML 欄の MML か overlay の素材）は書き換えず、鳴らす列だけを作り直す。声部は素材の全音の音高を
//! オクターブレンジぶん +12 ずつ複製し、oct シフトぶんずらしてから、低い順・重複なしに並べたもの。
//! 1 step の秒は、MML 素材なら素材の最初の列から次の列までの間隔、chord 素材なら設定の BPM と音価で、
//! 各音は 1 step ちょうど鳴らす。

use std::ops::RangeInclusive;

use cmrt_arpeggiator::{up_down_sequence, ArpPattern};
use serde::{Deserialize, Serialize};

use crate::{notes_from_events, Note, TimedMidiEvent};

mod rate;

pub use rate::ArpRate;

/// オクターブレンジ（声部集合を何オクターブぶん並べるか）の範囲。
pub const ARP_OCTAVES: RangeInclusive<usize> = 1..=3;
/// [`ArpPattern::UpDown`] の下り幅（最高音から下りる音数）の範囲。この先は「全部」（`None`）。
pub const ARP_DOWN: RangeInclusive<usize> = 1..=8;
/// oct シフト（声部全体を何オクターブずらすか）の範囲。
pub const ARP_SHIFT: RangeInclusive<i8> = -2..=2;
/// chord 素材の BPM の範囲。
pub const ARP_BPM: RangeInclusive<u16> = 40..=240;
/// アルペジオの各音の velocity。
const ARP_VELOCITY: u8 = 127;

/// この画面で選べる音型。声部の並べ替え（Converge・Diverge）・最高音との交互（Octave）・
/// 端の重ね（UpDownHold）・Random は持たない。
pub const ARP_PATTERNS: [ArpPattern; 4] = [
    ArpPattern::Up,
    ArpPattern::Down,
    ArpPattern::UpDown,
    ArpPattern::DownUp,
];

/// アルペジエーターの設定。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArpSettings {
    /// [`ArpPattern::label`] の綴りで読み書きする。[`ARP_PATTERNS`] に無い綴りは [`ArpPattern::Up`]。
    #[serde(with = "pattern_label")]
    pub pattern: ArpPattern,
    /// [`ARP_OCTAVES`] の値。
    pub octaves: usize,
    /// [`ArpPattern::UpDown`] の下り幅（[`ARP_DOWN`]）。`None` は 1 番の声部まで下りる。
    pub down: Option<usize>,
    /// 声部全体を 12 × `shift` 半音ずらす（[`ARP_SHIFT`]）。
    pub shift: i8,
    /// chord 素材の BPM（[`ARP_BPM`]）。MML 素材には効かない。
    pub bpm: u16,
    /// chord 素材の 1 step の音価。MML 素材には効かない。
    pub rate: ArpRate,
}

impl Default for ArpSettings {
    fn default() -> Self {
        Self {
            pattern: ArpPattern::Up,
            octaves: 1,
            down: None,
            shift: 0,
            bpm: 120,
            rate: ArpRate::Sixteenth,
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

/// 素材から作った、鳴らすイベント列。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialPerformance {
    pub events: Vec<TimedMidiEvent>,
    /// 素材が chord 表記として解釈されたか（[`cmrt_chord::TimedPerformance::from_chord`]）。
    pub from_chord: bool,
}

/// 素材から、鳴らすイベント列と、素材が chord 表記かを作る。空の MML は空、それ以外は
/// [`cmrt_chord::timed_performance`] に、`arp` があれば [`arpeggiate`] を当てたもの。
pub fn material_performance(
    mml: &str,
    arp: Option<&ArpSettings>,
) -> Result<MaterialPerformance, String> {
    if mml.is_empty() {
        return Ok(MaterialPerformance::default());
    }
    let performance = cmrt_chord::timed_performance(mml)?;
    let events = match arp {
        Some(arp) => arpeggiate(&performance.events, arp, performance.from_chord),
        None => performance.events,
    };
    Ok(MaterialPerformance {
        events,
        from_chord: performance.from_chord,
    })
}

/// [`material_performance`] のイベント列。
pub fn performance_events(
    mml: &str,
    arp: Option<&ArpSettings>,
) -> Result<Vec<TimedMidiEvent>, String> {
    material_performance(mml, arp).map(|performance| performance.events)
}

/// 素材の音を声部にしたアルペジオの note on / note off を返す。
///
/// 長さは音型の 1 周期。各音の channel は素材で同じ音高を最初に鳴らした音のもの、velocity は
/// 上限の 127（和音の素材は同時発音ぶん velocity が下げてあり、単音で鳴らすアルペジオには小さすぎる）。
/// `from_chord` なら 1 step は `bpm` と `rate` の音価、そうでなければ素材の刻み。
/// 範囲外の設定値は範囲の端へ寄せる。素材に音が無いか、刻みが 0 なら素材のまま。
pub fn arpeggiate(
    plain: &[TimedMidiEvent],
    settings: &ArpSettings,
    from_chord: bool,
) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(plain);
    let step = if from_chord {
        (!notes.is_empty()).then(|| chord_step_seconds(settings))
    } else {
        step_seconds(&notes)
    };
    let Some(step) = step.filter(|step| *step > 0.0) else {
        return plain.to_vec();
    };
    let shift = settings.shift.clamp(*ARP_SHIFT.start(), *ARP_SHIFT.end());
    let voices = voices(&notes, clamp(settings.octaves, ARP_OCTAVES), shift);
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
                    message: [0x90 | note.channel, note.pitch, ARP_VELOCITY],
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

/// chord 素材の 1 step の秒。BPM は [`ARP_BPM`] の端へ寄せる。
fn chord_step_seconds(settings: &ArpSettings) -> f64 {
    let bpm = settings.bpm.clamp(*ARP_BPM.start(), *ARP_BPM.end());
    settings.rate.seconds(bpm)
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

/// 素材の音高を `octaves` ぶん +12 ずつ複製し、12 × `shift` ずらして、低い順・重複なしに並べた声部。
/// 同じ音高は低いオクターブの複製（素材側）を残す。MIDI の音域を外れる音は落とす。
fn voices(notes: &[Note], octaves: usize, shift: i8) -> Vec<Note> {
    let mut base: Vec<Note> = Vec::new();
    for note in notes {
        if !base.iter().any(|voice| voice.pitch == note.pitch) {
            base.push(*note);
        }
    }
    let mut voices: Vec<Note> = (0..octaves as i16)
        .flat_map(|octave| {
            let offset = (octave + i16::from(shift)) * 12;
            base.iter().filter_map(move |note| {
                let pitch = u8::try_from(i16::from(note.pitch) + offset)
                    .ok()
                    .filter(|p| *p <= 127)?;
                Some(Note { pitch, ..*note })
            })
        })
        .collect();
    voices.sort_by_key(|note| note.pitch);
    voices.dedup_by_key(|note| note.pitch);
    voices
}

/// 音型の 1 周期の声部番号。
fn voice_order(settings: &ArpSettings, voice_count: usize) -> Vec<usize> {
    match settings.pattern {
        ArpPattern::UpDown => {
            up_down_sequence(voice_count, settings.down.map(|down| clamp(down, ARP_DOWN)))
        }
        pattern => pattern.voice_sequence(voice_count).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests;
