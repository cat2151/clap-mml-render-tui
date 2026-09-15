//! Chord Chart の preview 文脈から、選択 section 用の auto voicing を取り出す。

use cmrt_chord::{ChordVoicing, ParsedChordProgression};
use cmrt_chord_chart::PreviewVoicingContext;

/// Arrangement 内で読めない section があれば、そこを境にして選択 section を含む
/// 連続区間だけを voice する。壊れた別 section のせいで正常な preview まで失わないため。
pub(super) fn selected_voicings(
    key_token: Option<&str>,
    context: &PreviewVoicingContext,
) -> Option<Vec<ChordVoicing>> {
    let selected = context.selected;
    let parsed = context
        .progressions
        .iter()
        .map(|progression| parse(key_token, progression).ok())
        .collect::<Vec<_>>();
    let key_pitch_class = parsed.get(selected)?.as_ref()?.key_pitch_class();

    let run_start = parsed[..selected]
        .iter()
        .rposition(Option::is_none)
        .map_or(0, |index| index + 1);
    let run_end = parsed[selected + 1..]
        .iter()
        .position(Option::is_none)
        .map_or(parsed.len(), |offset| selected + 1 + offset);
    let selected_offset = parsed[run_start..selected]
        .iter()
        .flatten()
        .map(|progression| progression.chords().len())
        .sum::<usize>();
    let selected_len = parsed[selected].as_ref()?.chords().len();
    let chords = parsed[run_start..run_end]
        .iter()
        .flatten()
        .flat_map(|progression| progression.chords().iter().cloned())
        .collect::<Vec<_>>();
    let voiced = cmrt_chord::auto_voice_with_key(&chords, key_pitch_class, None);
    voiced
        .get(selected_offset..selected_offset + selected_len)
        .map(<[ChordVoicing]>::to_vec)
}

fn parse(key_token: Option<&str>, progression: &str) -> Result<ParsedChordProgression, String> {
    let key = key_token.unwrap_or("Key:C");
    cmrt_chord::parse_chord_progression(&format!("{key} {progression}"))
}

#[cfg(test)]
mod tests;
