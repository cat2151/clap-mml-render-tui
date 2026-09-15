//! Auto voicing と時刻つき chord performance の対応付け。

use std::collections::VecDeque;

use crate::{TimedMidiEvent, TimedPerformance};

use super::{auto_voice_with_key, ChordVoicing};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;

/// chord2mml が作った進行の発音タイミングを保ったまま、音高だけを指定 voicing へ置き換える。
///
/// `chord_index` があれば、その chord の note on/off だけを先頭時刻へ移して返す。
/// Chord Chart で進行全体の auto voicing を保ったまま 1 chord だけ試聴するための入口。
/// bass は別パート用なのでここでは加えず、[`ChordVoicing::notes`] だけを使う。
pub fn revoice_timed_progression(
    performance: TimedPerformance,
    voicings: &[ChordVoicing],
    chord_index: Option<usize>,
) -> Result<TimedPerformance, String> {
    let mut onset_groups: Vec<(f64, Vec<u8>)> = Vec::new();
    for event in &performance.events {
        if !is_note_on(event) {
            continue;
        }
        match onset_groups.last_mut() {
            Some((seconds, notes)) if *seconds == event.seconds => notes.push(event.message[1]),
            _ => onset_groups.push((event.seconds, vec![event.message[1]])),
        }
    }
    validate_counts(onset_groups.len(), voicings, chord_index)?;

    let mut pitch_maps = Vec::with_capacity(voicings.len());
    for ((_, source_notes), voicing) in onset_groups.iter().zip(voicings) {
        let mut source_notes = source_notes.clone();
        source_notes.sort_unstable();
        source_notes.dedup();
        let mut target_notes = voicing.notes.clone();
        target_notes.sort_unstable();
        target_notes.dedup();
        if source_notes.len() != target_notes.len() {
            return Err(format!(
                "構成音数と voicing の音数が一致しません（source={} target={}）",
                source_notes.len(),
                target_notes.len()
            ));
        }
        let mut map = [None; 128];
        for (source, target) in source_notes.into_iter().zip(target_notes) {
            map[usize::from(source)] = Some(target);
        }
        pitch_maps.push(map);
    }

    let selected_start = chord_index
        .map(|index| onset_groups[index].0)
        .unwrap_or(0.0);
    let mut active = [None; 128];
    let mut current_onset = None;
    let mut current_chord = 0_usize;
    let mut events = Vec::with_capacity(performance.events.len());
    for mut event in performance.events {
        let kind = event.message[0] & 0xf0;
        let source = usize::from(event.message[1]);
        let assignment = match kind {
            NOTE_ON if event.message[2] > 0 => {
                if current_onset != Some(event.seconds) {
                    current_chord = current_onset.map_or(0, |_| current_chord + 1);
                    current_onset = Some(event.seconds);
                }
                let target = pitch_maps
                    .get(current_chord)
                    .and_then(|map| map[source])
                    .ok_or_else(|| "発音ノートを voicing へ対応付けられません".to_string())?;
                active[source] = Some((target, current_chord));
                Some((target, current_chord))
            }
            NOTE_OFF | NOTE_ON => active[source].take(),
            _ => None,
        };
        let Some((target, event_chord)) = assignment else {
            continue;
        };
        event.message[1] = target;
        if chord_index.is_none_or(|selected| selected == event_chord) {
            event.seconds -= selected_start;
            events.push(event);
        }
    }

    let duration_seconds = match chord_index {
        Some(_) => events
            .iter()
            .map(|event| event.seconds)
            .fold(0.0_f64, f64::max),
        None => performance.duration_seconds,
    };
    Ok(TimedPerformance {
        events,
        duration_seconds,
        from_chord: performance.from_chord,
    })
}

/// 元の chord performance と同じ区間で、各 [`ChordVoicing::bass`] を1音ずつ鳴らす。
///
/// 元 performance は変更せず、同時刻の note-on 群を1 chord として数える。Bass の
/// velocity と MIDI channel は各群の先頭 note-on から取り、終了時刻はその群の全音が
/// release された時刻に合わせる。`chord_index` があれば対象区間だけを時刻0へ移す。
pub fn bass_timed_progression(
    performance: &TimedPerformance,
    voicings: &[ChordVoicing],
    chord_index: Option<usize>,
) -> Result<TimedPerformance, String> {
    let intervals = chord_intervals(performance)?;
    validate_counts(intervals.len(), voicings, chord_index)?;

    let selected_start = chord_index
        .map(|index| intervals[index].note_on.seconds)
        .unwrap_or(0.0);
    let mut events = Vec::with_capacity(voicings.len() * 2);
    for (index, (interval, voicing)) in intervals.iter().zip(voicings).enumerate() {
        if chord_index.is_some_and(|selected| selected != index) {
            continue;
        }
        let Some(bass) = voicing.bass else {
            continue;
        };
        let mut note_on = interval.note_on;
        note_on.seconds -= selected_start;
        note_on.message[1] = bass;
        let mut note_off = interval
            .note_off
            .expect("chord_intervals rejects an onset without a note-off");
        note_off.seconds -= selected_start;
        note_off.message[1] = bass;
        events.extend([note_on, note_off]);
    }
    events.sort_by(|left, right| {
        left.seconds
            .total_cmp(&right.seconds)
            .then_with(|| is_note_on(left).cmp(&is_note_on(right)))
    });

    let duration_seconds = chord_index.map_or(performance.duration_seconds, |index| {
        intervals[index]
            .note_off
            .expect("chord_intervals rejects an onset without a note-off")
            .seconds
            - intervals[index].note_on.seconds
    });
    Ok(TimedPerformance {
        events,
        duration_seconds,
        from_chord: performance.from_chord,
    })
}

/// Chord Chart の 1 section を parse・auto voice・時刻つき演奏へ一度に変換する。
///
/// Key が省略されたときは chord2mml の既定と同じ C を使う。`chord_index` を指定しても
/// voicing 自体は section 全体から決めるため、単独試聴だけ転回形が戻ることはない。
pub fn timed_auto_voiced_chord_progression_performance(
    key_token: Option<&str>,
    progression: &str,
    chord_index: Option<usize>,
) -> Result<TimedPerformance, String> {
    let (performance, voicings) = auto_voiced_progression(key_token, progression)?;
    revoice_timed_progression(performance, &voicings, chord_index)
}

/// Chord Chart の 1 section を parse・auto voice し、Bass パートだけを時刻つき演奏へ
/// 変換する。
///
/// [`timed_auto_voiced_chord_progression_performance`] と同じ voicing を使うため、音色選択の
/// Bass 試聴でも Chord 側と対応する root と octave を保つ。
pub fn timed_auto_voiced_bass_chord_progression_performance(
    key_token: Option<&str>,
    progression: &str,
    chord_index: Option<usize>,
) -> Result<TimedPerformance, String> {
    let (performance, voicings) = auto_voiced_progression(key_token, progression)?;
    bass_timed_progression(&performance, &voicings, chord_index)
}

fn auto_voiced_progression(
    key_token: Option<&str>,
    progression: &str,
) -> Result<(TimedPerformance, Vec<ChordVoicing>), String> {
    let key = key_token.unwrap_or("Key:C");
    let parsed = crate::parse_chord_progression(&format!("{key} {progression}"))?;
    let voicings = auto_voice_with_key(parsed.chords(), parsed.key_pitch_class(), None);
    let performance = crate::timed_chord_progression_performance(key_token, progression)?;
    Ok((performance, voicings))
}

#[derive(Clone, Copy, Debug)]
struct ChordInterval {
    note_on: TimedMidiEvent,
    note_off: Option<TimedMidiEvent>,
    active_notes: usize,
}

fn chord_intervals(performance: &TimedPerformance) -> Result<Vec<ChordInterval>, String> {
    let mut intervals: Vec<ChordInterval> = Vec::new();
    let mut active: [VecDeque<usize>; 128] = std::array::from_fn(|_| VecDeque::new());
    let mut current_onset = None;

    for event in &performance.events {
        let pitch = usize::from(event.message[1]);
        if is_note_on(event) {
            let interval_index = if current_onset == Some(event.seconds) {
                intervals.len() - 1
            } else {
                current_onset = Some(event.seconds);
                intervals.push(ChordInterval {
                    note_on: *event,
                    note_off: None,
                    active_notes: 0,
                });
                intervals.len() - 1
            };
            intervals[interval_index].active_notes += 1;
            active[pitch].push_back(interval_index);
        } else if is_note_off(event) {
            let Some(interval_index) = active[pitch].pop_front() else {
                continue;
            };
            let interval = &mut intervals[interval_index];
            interval.active_notes -= 1;
            if interval
                .note_off
                .is_none_or(|note_off| event.seconds > note_off.seconds)
            {
                interval.note_off = Some(*event);
            }
        }
    }

    if let Some(index) = intervals
        .iter()
        .position(|interval| interval.active_notes != 0 || interval.note_off.is_none())
    {
        return Err(format!(
            "chord の note-off を対応付けられません（chord={index}）"
        ));
    }
    Ok(intervals)
}

fn validate_counts(
    onset_count: usize,
    voicings: &[ChordVoicing],
    chord_index: Option<usize>,
) -> Result<(), String> {
    if onset_count != voicings.len() {
        return Err(format!(
            "コード数と voicing 数が一致しません（events={} voicings={}）",
            onset_count,
            voicings.len()
        ));
    }
    if chord_index.is_some_and(|index| index >= voicings.len()) {
        return Err("試聴する chord の番号が範囲外です".to_string());
    }
    Ok(())
}

fn is_note_on(event: &TimedMidiEvent) -> bool {
    event.message[0] & 0xf0 == NOTE_ON && event.message[2] > 0
}

fn is_note_off(event: &TimedMidiEvent) -> bool {
    let kind = event.message[0] & 0xf0;
    kind == NOTE_OFF || (kind == NOTE_ON && event.message[2] == 0)
}

#[cfg(test)]
mod tests;
