use std::time::Instant;

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::note::midi_note_name;
use crate::state::arp_sequence;
use crate::{KeyboardState, KEYBOARD_NOTES};
use cmrt_tui_core::status::base_style;
use cmrt_tui_core::theme::MONOKAI_GREEN;

const PC_KEY_LABEL: &str = "PC key:  ";
const NOTE_LABEL: &str = "Note:    ";
const MIN_COLUMN_WIDTH: usize = 4;

/// PC key 行と Note 行。列は PC キーの 7 音と進行全体の音を音高順に並べたもので、
/// `now` の時点で鳴っている音（押さえている PC キー・repeat/arp の発音）に色を付ける。
pub(super) fn note_column_lines(state: &KeyboardState, now: Instant) -> [Line<'static>; 2] {
    let sounding = sounding_notes(state, now);
    let columns = note_columns(state, now, &sounding);
    let width = column_width(&columns);

    let mut keys = vec![Span::styled(PC_KEY_LABEL, base_style())];
    let mut notes = vec![Span::styled(NOTE_LABEL, base_style())];
    for &midi_note in &columns {
        let key = KEYBOARD_NOTES
            .iter()
            .find(|note| note.midi_note == midi_note)
            .map(|note| note.key.to_string())
            .unwrap_or_default();
        let held = state.held().iter().any(|note| note.midi_note == midi_note);
        push_cell(&mut keys, key, width, held);
        push_cell(
            &mut notes,
            midi_note_name(midi_note),
            width,
            sounding[usize::from(midi_note)],
        );
    }
    [Line::from(keys), Line::from(notes)]
}

/// PC key 行と Note 行の表示幅。発音中の音は数えないので、鳴らしている間も幅は変わらない。
pub(super) fn note_columns_width(state: &KeyboardState, now: Instant) -> usize {
    let columns = note_columns(state, now, &[false; 128]);
    NOTE_LABEL.len() + column_width(&columns) * columns.len()
}

fn column_width(columns: &[u8]) -> usize {
    columns
        .iter()
        .map(|&note| midi_note_name(note).len() + 1)
        .max()
        .unwrap_or(0)
        .max(MIN_COLUMN_WIDTH)
}

fn push_cell(spans: &mut Vec<Span<'static>>, text: String, width: usize, lit: bool) {
    let padding = " ".repeat(width.saturating_sub(text.chars().count()));
    let style = if lit { lit_style() } else { base_style() };
    spans.push(Span::styled(text, style));
    spans.push(Span::styled(padding, base_style()));
}

pub(super) fn lit_style() -> Style {
    base_style().fg(MONOKAI_GREEN).add_modifier(Modifier::BOLD)
}

/// `now` の時点で鳴っている音を MIDI ノート番号で引ける表にする。
fn sounding_notes(state: &KeyboardState, now: Instant) -> [bool; 128] {
    let mut sounding = [false; 128];
    for note in state.held() {
        sounding[usize::from(note.midi_note)] = true;
    }
    if let Some(position) = state.sounding_position(now) {
        match position.arp {
            Some(step) => sounding[usize::from(step.note.midi_note)] = true,
            None => {
                let chord = state.repeat_chords_at(now).get(position.chord_index);
                for note in chord.into_iter().flatten() {
                    sounding[usize::from(note.midi_note)] = true;
                }
            }
        }
    }
    sounding
}

/// 列に並べる音。鳴っている音は進行から外れていても列に入れ、色を付ける場所を必ず持たせる。
fn note_columns(state: &KeyboardState, now: Instant, sounding: &[bool; 128]) -> Vec<u8> {
    let mut present = *sounding;
    for note in KEYBOARD_NOTES {
        present[usize::from(note.midi_note)] = true;
    }
    for chord in state.repeat_chords_at(now) {
        let notes = if state.note_playback_uses_arp() {
            arp_sequence(chord)
        } else {
            chord.clone()
        };
        for note in notes {
            present[usize::from(note.midi_note)] = true;
        }
    }
    (0u8..128)
        .filter(|&note| present[usize::from(note)])
        .collect()
}
