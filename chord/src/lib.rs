//! コード表記（chord name / degree）から note number 群への変換と、コード進行カタログ。
//!
//! 変換は `chord2mml → MML → SMF → NoteOn 収集` という経路をたどる。chord2mml が
//! note number を直接返す公開 API を持たないため、SMF を経由するのが唯一の手段。
//!
//! keyboard 画面の MML 入力と grid sequencer の chord mode の両方から使う。

mod auto_voicing;
mod progression;

pub use auto_voicing::{auto_voice, max_jumps, ChordVoicing};
pub use progression::{
    chord_notes, parse_chord_progression, ChordProgression, ChordProgressionCatalog,
    ChordProgressionPick, ParsedChordProgression, KEYS,
};

use midly::{MidiMessage, Smf, TrackEventKind};

/// コード表記または MML を、同時発音ごとにまとめた note number の列へ変換する。
///
/// chord2mml の変換に失敗した入力は、そのまま生 MML として扱う（keyboard 画面が
/// コード表記と MML の両方を1つの入力欄で受け付けるための仕様）。この
/// フォールバックが要らない呼び出し側は [`mml_note_progression`] を使うこと。
pub fn note_progression(input: &str) -> Result<Vec<Vec<u8>>, String> {
    if input.trim().is_empty() {
        return Err("MMLを入力してください".to_string());
    }
    let mml = chord2mml_core::convert(input).unwrap_or_else(|_| input.to_string());
    mml_note_progression(&mml)
}

/// MML を、同時発音ごとにまとめた note number の列へ変換する。
pub fn mml_note_progression(mml: &str) -> Result<Vec<Vec<u8>>, String> {
    let bytes = mmlabc_to_smf::mml_to_smf_bytes(mml)
        .map_err(|error| format!("MML変換に失敗しました: {error}"))?;
    note_progression_from_smf(&bytes)
}

/// SMF の NoteOn を絶対時刻順に集め、同じ時刻のものを1つの和音へまとめる。
fn note_progression_from_smf(bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let smf = Smf::parse(bytes).map_err(|error| format!("SMF解析に失敗しました: {error}"))?;
    let mut note_ons = Vec::new();
    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut absolute_time = 0_u64;
        for (event_index, event) in track.iter().enumerate() {
            absolute_time = absolute_time.saturating_add(u64::from(event.delta.as_int()));
            if let TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } = event.kind
            {
                if vel.as_int() > 0 {
                    note_ons.push((absolute_time, track_index, event_index, key.as_int()));
                }
            }
        }
    }
    note_ons.sort_unstable_by_key(|&(time, track, event, _)| (time, track, event));

    let mut progression = Vec::new();
    let mut chord_time = None;
    let mut seen = [false; 128];
    for (time, _, _, note) in note_ons {
        if chord_time != Some(time) {
            progression.push(Vec::new());
            chord_time = Some(time);
            seen = [false; 128];
        }
        if !seen[usize::from(note)] {
            seen[usize::from(note)] = true;
            progression
                .last_mut()
                .expect("a chord exists for every note-on")
                .push(note);
        }
    }
    if progression.is_empty() {
        return Err("MMLに発音ノートがありません".to_string());
    }
    Ok(progression)
}

#[cfg(test)]
mod tests;
