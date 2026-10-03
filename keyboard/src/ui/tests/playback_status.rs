use std::time::{Duration, Instant};

use super::note_columns::{cells, chord_state, cycle_to, lit, names, pump, render_at, TICK};
use super::*;
use crate::periodic_timeline::LOOKAHEAD;
use crate::NotePlaybackMode;

fn status_cells(terminal: &Terminal<TestBackend>, label: &str) -> Vec<(String, Color)> {
    cells(terminal, &format!("│{label} "))
}

/// 和音 C4 G4 の arp を、G4（arp 列の 2 音目）の tick まで進める。
fn arp_at_g4(t0: Instant) -> (KeyboardState, Instant) {
    let mut state = chord_state(vec![vec![67, 60]], t0);
    cycle_to(&mut state, NotePlaybackMode::Arp, t0);
    let deadline = pump(&mut state, t0 + TICK - LOOKAHEAD);
    (state, deadline)
}

#[test]
fn arp_shows_the_arp_sequence_and_lights_the_sounding_note() {
    let (state, deadline) = arp_at_g4(Instant::now());

    let terminal = render_at(state, deadline);

    let cells = status_cells(&terminal, "Arp:");
    assert_eq!(names(&cells), ["C4", "G4", "C5", "G5"]);
    assert_eq!(lit(&cells), ["G4"]);
}

#[test]
fn the_arp_status_follows_the_real_sound_time_not_the_lookahead() {
    let (state, deadline) = arp_at_g4(Instant::now());

    let terminal = render_at(state, deadline - Duration::from_millis(1));

    assert_eq!(lit(&status_cells(&terminal, "Arp:")), ["C4"]);
}

#[test]
fn repeat_lights_the_sounding_chord_of_the_progression() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 64], vec![65, 69]], t0);
    cycle_to(&mut state, NotePlaybackMode::Repeat, t0);
    let mut deadline = t0;
    for step in 1..=32u32 {
        deadline = pump(&mut state, t0 + TICK * step - LOOKAHEAD);
        if state
            .sounding_position(deadline)
            .is_some_and(|position| position.chord_index == 1)
        {
            break;
        }
    }

    let terminal = render_at(state, deadline);

    let cells = status_cells(&terminal, "Repeat:");
    assert_eq!(names(&cells), ["C4", "E4", "|", "F4", "A4"]);
    assert_eq!(lit(&cells), ["F4", "A4"]);
}

#[test]
fn repeat_lights_the_first_chord_right_after_it_starts() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 64], vec![65, 69]], t0);
    cycle_to(&mut state, NotePlaybackMode::Repeat, t0);

    let terminal = render_at(state, t0);

    assert_eq!(lit(&status_cells(&terminal, "Repeat:")), ["C4", "E4"]);
}

#[test]
fn off_names_the_chord_without_color_and_target_is_gone() {
    let t0 = Instant::now();
    let state = chord_state(vec![vec![60, 67]], t0);

    let terminal = render_at(state, t0);

    let cells = status_cells(&terminal, "Chord:");
    assert_eq!(names(&cells), ["C4", "G4"]);
    assert!(lit(&cells).is_empty(), "{cells:?}");
    assert!(!buffer_to_string(&terminal).contains("Target:"));
}
