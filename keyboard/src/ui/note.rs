use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::{KeyboardState, NotePlaybackMode};
use cmrt_tui_core::status::base_style;
use cmrt_tui_core::theme::{MONOKAI_GRAY, MONOKAI_GREEN};

pub(super) fn midi_note_name(note: u8) -> String {
    const PITCH_CLASSES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pitch_class = PITCH_CLASSES[usize::from(note % 12)];
    let octave = i16::from(note / 12) - 1;
    format!("{pitch_class}{octave}")
}

pub(super) fn note_playback_status_text(state: &KeyboardState) -> String {
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
    format!("Target: {}", if names.is_empty() { "-" } else { &names })
}

/// `t` の欄の 1 行。全モードを巡回順に並べ、今のモードだけ色を付ける。
pub(super) fn note_playback_mode_line(state: &KeyboardState) -> Line<'static> {
    const ORDER: [NotePlaybackMode; 4] = [
        NotePlaybackMode::Off,
        NotePlaybackMode::Auto,
        NotePlaybackMode::Repeat,
        NotePlaybackMode::Arp,
    ];
    let current = state.note_playback_mode();
    let mut spans = vec![Span::styled("t: repeat: ", base_style())];
    for (index, mode) in ORDER.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ", base_style()));
        }
        let label = match mode {
            NotePlaybackMode::Off => "off",
            NotePlaybackMode::Auto if mode == current && state.note_playback_uses_arp() => {
                "auto→arp"
            }
            NotePlaybackMode::Auto if mode == current => "auto→repeat",
            NotePlaybackMode::Auto => "auto",
            NotePlaybackMode::Repeat => "repeat",
            NotePlaybackMode::Arp => "arp",
        };
        let style = if mode == current {
            base_style().fg(MONOKAI_GREEN).add_modifier(Modifier::BOLD)
        } else {
            base_style().fg(MONOKAI_GRAY)
        };
        spans.push(Span::styled(label, style));
    }
    Line::from(spans)
}
