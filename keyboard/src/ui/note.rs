use ratatui::text::{Line, Span};

use std::time::Instant;

use super::note_columns::lit_style;
use crate::state::{arp_sequence, PlaybackNote};
use crate::{ArpStep, KeyboardState, NotePlaybackMode};
use cmrt_tui_core::status::base_style;

pub(super) fn midi_note_name(note: u8) -> String {
    const PITCH_CLASSES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pitch_class = PITCH_CLASSES[usize::from(note % 12)];
    let octave = i16::from(note / 12) - 1;
    format!("{pitch_class}{octave}")
}

/// `t` で鳴らす和音の行。ラベルは実効モード（arp / repeat / off）で変わり、
/// arp は各和音を arp 列で並べていま鳴っている音に、repeat はいま鳴っている和音に色を付ける。
pub(super) fn note_playback_status_line(state: &KeyboardState, now: Instant) -> Line<'static> {
    let uses_arp = state.note_playback_uses_arp();
    let label = if uses_arp {
        "Arp: "
    } else if state.note_playback_mode() == NotePlaybackMode::Off {
        "Chord: "
    } else {
        "Repeat: "
    };
    let mut spans = vec![Span::styled(label, base_style())];
    if state.repeat_chords_at(now).is_empty() {
        spans.push(Span::styled("-", base_style()));
        return Line::from(spans);
    }
    let position = state.sounding_position(now);
    for (chord_index, chord) in state.repeat_chords_at(now).iter().enumerate() {
        if chord_index > 0 {
            spans.push(Span::styled(" | ", base_style()));
        }
        let sounding = position.filter(|position| position.chord_index == chord_index);
        let notes = if uses_arp {
            arp_sequence(chord)
        } else {
            chord.clone()
        };
        let lit_index = sounding.and_then(|position| match position.arp {
            Some(step) => lit_arp_index(&notes, step),
            None => None,
        });
        let whole_chord_lit = sounding.is_some_and(|position| position.arp.is_none());
        for (index, note) in notes.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled(" ", base_style()));
            }
            let lit = whole_chord_lit || lit_index == Some(index);
            let style = if lit { lit_style() } else { base_style() };
            spans.push(Span::styled(midi_note_name(note.midi_note), style));
        }
    }
    Line::from(spans)
}

/// arp 列の中で色を付ける位置。発音中の音と index が食い違うとき（arp 中に和音へ音を足した直後）は、
/// 同じ音の最初の位置に付けて実音と合わせる。
fn lit_arp_index(notes: &[PlaybackNote], step: ArpStep) -> Option<usize> {
    let matches = |note: &PlaybackNote| note.midi_note == step.note.midi_note;
    if notes.get(step.index).is_some_and(matches) {
        return Some(step.index);
    }
    notes.iter().position(matches)
}

/// `t` の行。全モードを巡回順に並べ、今のモードだけ色を付ける。
pub(super) fn note_playback_mode_line(state: &KeyboardState) -> Line<'static> {
    const ORDER: [NotePlaybackMode; 4] = [
        NotePlaybackMode::Off,
        NotePlaybackMode::Auto,
        NotePlaybackMode::Repeat,
        NotePlaybackMode::Arp,
    ];
    let current = state.note_playback_mode();
    let mut spans = super::shortcut_label("t", 't');
    spans.push(Span::styled(": ", base_style()));
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
        spans.push(Span::styled(label, super::choice_style(mode == current)));
    }
    Line::from(spans)
}
