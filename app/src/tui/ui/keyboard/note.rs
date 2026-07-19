use crate::tui::keyboard::{KeyboardState, NotePlaybackMode};

pub(super) fn midi_note_name(note: u8) -> String {
    const PITCH_CLASSES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pitch_class = PITCH_CLASSES[usize::from(note % 12)];
    let octave = i16::from(note / 12) - 1;
    format!("{pitch_class}{octave}")
}

pub(super) fn note_playback_status_text(state: &KeyboardState) -> String {
    let mode = match state.note_playback_mode() {
        NotePlaybackMode::Off => "off",
        NotePlaybackMode::Repeat => "repeat",
        NotePlaybackMode::Arp => "arp",
        NotePlaybackMode::Auto if state.note_playback_uses_arp() => "auto→arp",
        NotePlaybackMode::Auto => "auto→repeat",
    };
    let names = state
        .repeat_chords()
        .iter()
        .map(|chord| {
            let mut notes = chord.to_vec();
            if state.note_playback_uses_arp() {
                notes.sort_unstable_by_key(|note| note.midi_note);
            }
            notes
                .iter()
                .map(|note| midi_note_name(note.midi_note))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ");
    if state.note_playback_mode() == NotePlaybackMode::Off {
        return format!(
            "Note mode: off | Target: {}",
            if names.is_empty() { "-" } else { &names }
        );
    }
    format!("Note mode: {mode} {names}")
}
