use super::*;
use crate::ChordPlayback;

#[test]
fn a_half_measure_output_drop_keeps_the_display_at_the_audible_measure_head() {
    let started = Instant::now();
    let wall_now = started + crate::step_offset(24);
    let audible_now = started + crate::step_offset(16);
    let mut state = GridState::silent();
    state.start(started);

    state.poll_steps_at(started, wall_now - started, audible_now);

    assert_eq!(state.displayed_ordinal(), Some(16));
    assert_eq!(state.step_index(), 0);
}

#[test]
fn the_uncompensated_wall_clock_would_be_half_a_measure_ahead() {
    let started = Instant::now();
    let wall_now = started + crate::step_offset(24);
    let mut state = GridState::silent();
    state.start(started);

    state.poll_steps_at(started, wall_now - started, wall_now);

    assert_eq!(state.displayed_ordinal(), Some(24));
    assert_eq!(state.step_index(), 8);
}

#[test]
fn chord_off_records_each_sixteen_step_loop_at_its_audible_head() {
    let started = Instant::now();
    let mut state = GridState::silent();
    state.start(started);

    state.poll_steps(started, Duration::ZERO);
    assert_eq!(state.take_history_snapshots().len(), 1);

    for step in 1..crate::GRID_STEPS as u64 {
        state.poll_steps(started + crate::step_offset(step), Duration::ZERO);
        assert!(state.take_history_snapshots().is_empty());
    }
    state.poll_steps(
        started + crate::step_offset(crate::GRID_STEPS as u64),
        Duration::ZERO,
    );
    assert_eq!(state.take_history_snapshots().len(), 1);
}

#[test]
fn chord_mode_records_only_at_the_full_progression_boundary() {
    let started = Instant::now();
    let chord = ChordPlayback::new(
        "C",
        "I-V".to_string(),
        vec![vec![60, 64, 67], vec![67, 71, 74]],
    )
    .unwrap();
    let mut state = GridState::silent();
    state.set_chord(Some(chord), started);
    state.start(started);

    state.poll_steps(started, Duration::ZERO);
    assert_eq!(state.take_history_snapshots()[0].measure_count(), 2);

    for step in 1..=crate::GRID_STEPS as u64 {
        state.poll_steps(started + crate::step_offset(step), Duration::ZERO);
    }
    assert!(state.take_history_snapshots().is_empty());

    for step in crate::GRID_STEPS as u64 + 1..=(crate::GRID_STEPS * 2) as u64 {
        state.poll_steps(started + crate::step_offset(step), Duration::ZERO);
    }
    assert_eq!(state.take_history_snapshots().len(), 1);
}
