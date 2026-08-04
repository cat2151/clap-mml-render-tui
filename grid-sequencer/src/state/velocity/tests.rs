use std::time::{Duration, Instant};

use super::super::StepDuration;
use super::*;
use crate::{step_offset, ChordPlayback, GRID_STEPS};

fn note_ons_at(state: &mut GridState, now: Instant) -> Vec<[u8; 3]> {
    state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .map(|scheduled| scheduled.message)
        .filter(|message| message[0] == 0x90)
        .collect()
}

#[test]
fn note_on_carries_the_velocity_drawn_for_that_cell() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].cells[0] = true;
    state.start(now);

    let note_ons = note_ons_at(&mut state, now);

    assert_eq!(note_ons.len(), 1);
    assert!(matches!(note_ons[0], [0x90, 60, 100 | 127]));
    assert_eq!(state.velocity_display()[0][0], Some(note_ons[0][2]));
    assert!(state.velocity_display()[0][1..].iter().all(Option::is_none));
}

/// 和音は1回の attack なので、構成音すべてが同じ velocity で鳴る。
#[test]
fn a_chord_sounds_every_note_with_the_same_velocity() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    state.set_chord(Some(chord), now);
    state.start(now);

    let note_ons = note_ons_at(&mut state, now);

    assert_eq!(note_ons.len(), 3);
    let velocity = note_ons[0][2];
    assert!(VELOCITY_CHOICES.contains(&velocity));
    assert!(
        note_ons.iter().all(|message| message[2] == velocity),
        "{note_ons:?}"
    );
}

/// 抽選は小節頭だけ。同じ小節の中では値が固定される。
#[test]
fn the_measure_is_redrawn_only_at_its_head() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].cells.fill(true);
    state.start(now);

    let mut measure = Vec::new();
    for step in 0..GRID_STEPS as u64 {
        measure.push(note_ons_at(&mut state, now + step_offset(step))[0][2]);
        assert_eq!(
            state.velocity_display()[0][step as usize],
            Some(measure[step as usize]),
            "step {step}"
        );
    }
    let head_of_next = note_ons_at(&mut state, now + step_offset(GRID_STEPS as u64))[0][2];

    assert!(measure.iter().all(|value| VELOCITY_CHOICES.contains(value)));
    assert!(VELOCITY_CHOICES.contains(&head_of_next));
}

#[test]
fn a_sustained_note_is_not_retriggered_mid_measure() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].duration = StepDuration::Quarter;
    state.rows[0].cells[0] = true;
    state.start(now);

    let first = note_ons_at(&mut state, now);
    let second = note_ons_at(&mut state, now + step_offset(1));

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
}
