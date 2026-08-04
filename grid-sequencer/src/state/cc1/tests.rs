use std::time::{Duration, Instant};

use super::super::StepDuration;
use super::*;
use crate::{step_offset, ChordPlayback, GRID_STEPS};

fn messages_at(state: &mut GridState, now: Instant) -> Vec<[u8; 3]> {
    state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .map(|scheduled| scheduled.message)
        .collect()
}

fn cc1_messages_at(state: &mut GridState, now: Instant) -> Vec<(u8, u8)> {
    state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .filter(|scheduled| scheduled.message[0] == CONTROL_CHANGE)
        .map(|scheduled| (scheduled.instance_id, scheduled.message[2]))
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

/// modulation は発音中にも効くので、鳴らないステップでも送る。
#[test]
fn silent_steps_still_send_cc1() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.start(now);

    assert_eq!(cc1_messages_at(&mut state, now).len(), 1);
    assert_eq!(cc1_messages_at(&mut state, now + step_offset(1)).len(), 1);
}

/// 送るのは全行ぶん。譜面が空の行にも毎ステップ送る。
#[test]
fn every_row_gets_a_cc1_on_every_step() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(3);
    state.rows[0].cells[0] = true;
    state.start(now);

    for step in 0..GRID_STEPS as u64 {
        let sent = cc1_messages_at(&mut state, now + step_offset(step));
        let instances = sent
            .iter()
            .map(|(instance, _)| *instance)
            .collect::<Vec<_>>();
        assert_eq!(instances, vec![0, 1, 2], "step {step}");
    }
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

/// CC1 grid は全stepで送る値をそのまま見せる。無音セルも埋まる。
///
/// ランプがどの区間に張られるかは [`super::super::measure_lane`] のテストで見る。
#[test]
fn the_display_covers_every_step_of_the_measure() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.rows[0].cells[0] = true;
    state.rows[0].cells[4] = true;
    state.start(now);
    messages_at(&mut state, now);

    let display = &state.cc1_display()[0];

    assert!(display.iter().all(Option::is_some), "{display:?}");
    // 小節頭は最初の note on の位置なので、ランプなら必ず2値のどちらか。
    assert!(matches!(display[0], Some(0 | 127)));
}
