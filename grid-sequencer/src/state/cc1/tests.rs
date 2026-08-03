use std::time::{Duration, Instant};

use super::*;
use crate::{step_offset, ChordPlayback};

fn messages_at(state: &mut GridState, now: Instant) -> Vec<[u8; 3]> {
    state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .map(|scheduled| scheduled.message)
        .collect()
}

#[test]
fn note_on_is_preceded_by_cc1_from_the_measure_plan() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].cells[0] = true;
    state.start(now);

    let messages = messages_at(&mut state, now);

    assert_eq!(messages.len(), 2);
    assert!(matches!(messages[0], [0xB0, 1, 0 | 127]));
    assert_eq!(messages[1], [0x90, 60, 100]);
    assert_eq!(state.cc1_display()[0][0], Some(messages[0][2]));
    assert!(state.cc1_display()[0][1..].iter().all(Option::is_none));
}

#[test]
fn retrigger_orders_cc1_before_note_off_and_note_on() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].duration = StepDuration::Quarter;
    state.rows[0].cells[0] = true;
    state.rows[0].cells[1] = true;
    state.start(now);
    messages_at(&mut state, now);

    let messages = messages_at(&mut state, now + step_offset(1));

    assert!(matches!(messages[0], [0xB0, 1, 0 | 127]));
    assert_eq!(messages[1], [0x80, 60, 0]);
    assert_eq!(messages[2], [0x90, 60, 100]);
}

#[test]
fn silent_steps_do_not_send_cc1() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.start(now);

    assert!(messages_at(&mut state, now).is_empty());
    assert!(state.cc1_display()[0].iter().all(Option::is_none));
    assert!(state.cc1.values[0]
        .iter()
        .all(|value| matches!(value, 0 | 127)));
}

#[test]
fn chord_attack_uses_one_cc1_for_all_simultaneous_notes() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    state.set_chord(Some(chord), now);
    state.start(now);

    let messages = messages_at(&mut state, now);

    assert_eq!(messages.len(), 4);
    assert!(matches!(messages[0], [0xB0, 1, 0 | 127]));
    assert_eq!(
        &messages[1..],
        &[[0x90, 60, 100], [0x90, 64, 100], [0x90, 67, 100]]
    );
}

#[test]
fn lookahead_does_not_reveal_the_next_measure_before_its_deadline() {
    let now = Instant::now();
    let deadline = now + step_offset(1);
    let mut state = GridState::with_row_count(1);
    state.cc1.display[0][0] = Some(0);
    let mut next = vec![[None; GRID_STEPS]];
    next[0][0] = Some(127);
    state.cc1.pending_display.push_back((deadline, next));

    state.advance_cc1_display(now);
    assert_eq!(state.cc1_display()[0][0], Some(0));

    state.advance_cc1_display(deadline);
    assert_eq!(state.cc1_display()[0][0], Some(127));
}

#[test]
fn a_mid_measure_score_change_reuses_the_values_drawn_at_the_measure_start() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].cells[0] = true;
    state.start(now);
    messages_at(&mut state, now);
    let already_drawn = state.cc1.values[0][3];

    state.rows[0].cells[0] = false;
    state.rows[0].cells[3] = true;
    state.refresh_cc1_display_pattern();

    assert_eq!(state.cc1_display()[0][0], None);
    assert_eq!(state.cc1_display()[0][3], Some(already_drawn));
}
