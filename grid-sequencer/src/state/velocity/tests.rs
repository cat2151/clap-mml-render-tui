use std::time::{Duration, Instant};

use super::*;
use crate::{step_offset, ChordPlayback, NotePattern, NoteStep, GRID_STEPS};

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
    state.instances[0].pattern.draw_span(0, 0);
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

#[test]
fn chord_voice_attacks_use_the_velocity_of_each_stored_lane() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(3);
    for lane in &mut state.instances[2].lanes {
        lane.pattern.draw_span(0, 0);
    }
    state.set_chord(
        ChordPlayback::new("C", "I7".to_string(), vec![vec![60, 64, 67, 71]]),
        now,
    );
    state.start(now);

    let note_ons = state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .filter(|message| message.instance_id == 2 && message.message[0] == 0x90)
        .map(|message| message.message)
        .collect::<Vec<_>>();
    assert_eq!(note_ons.len(), 4);
    for (lane, message) in note_ons.iter().enumerate() {
        let stored = state
            .stored_lane_index(crate::LaneAddress::new(2, lane))
            .expect("stored lane");
        assert_eq!(
            message[2],
            state.velocity_display()[stored][0].unwrap(),
            "lane {lane}"
        );
    }
}

/// 抽選は小節頭だけ。同じ小節の中では値が固定される。
#[test]
fn the_measure_is_redrawn_only_at_its_head() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.instances[0].pattern = NotePattern::from_steps([NoteStep::Attack; GRID_STEPS]);
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

    // ランプの補間値もありうるので、途中の値は範囲だけを見る。小節頭は必ず端の値。
    let range = VELOCITY_CHOICES[0]..=VELOCITY_CHOICES[1];
    assert!(
        measure.iter().all(|value| range.contains(value)),
        "{measure:?}"
    );
    assert!(VELOCITY_CHOICES.contains(&head_of_next));
}

#[test]
fn a_sustained_note_is_not_retriggered_mid_measure() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.instances[0].pattern.draw_span(0, 3);
    state.start(now);

    let first = note_ons_at(&mut state, now);
    let second = note_ons_at(&mut state, now + step_offset(1));

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
}
