use super::*;

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
