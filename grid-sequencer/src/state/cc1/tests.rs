use std::time::{Duration, Instant};

use super::super::StepDuration;
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
    assert!(matches!(messages[1], [0x90, 60, _]));
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

    // 小節の途中はランプの補間値もありうるので、値そのものは問わない。
    assert!(matches!(messages[0], [0xB0, 1, _]));
    assert_eq!(messages[1], [0x80, 60, 0]);
    assert!(matches!(messages[2], [0x90, 60, _]));
}

#[test]
fn silent_steps_do_not_send_cc1() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.start(now);

    assert!(messages_at(&mut state, now).is_empty());
    assert!(state.cc1_display()[0].iter().all(Option::is_none));
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
    assert!(messages[1..]
        .iter()
        .all(|message| matches!(message, [0x90, 60 | 64 | 67, _])));
}

/// CC1 grid は「実際に鳴るセル」だけを見せる。無音セルの抽選値は表に出ない。
#[test]
fn the_display_covers_the_whole_measure_of_sounding_cells() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].cells[0] = true;
    state.rows[0].cells[4] = true;
    state.start(now);
    messages_at(&mut state, now);

    let display = &state.cc1_display()[0];

    // 小節頭はランプの端なので必ず2値のどちらか。step 4 は補間値もありうる。
    assert!(matches!(display[0], Some(0 | 127)));
    assert!(display[4].is_some());
    assert_eq!(
        display.iter().filter(|value| value.is_some()).count(),
        2,
        "{display:?}"
    );
}
