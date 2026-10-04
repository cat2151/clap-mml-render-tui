use std::time::{Duration, Instant};

use super::*;

/// 何も鳴らしていない既定の状態では、keyboard pane は上限の幅まで広がらない。
#[test]
fn the_pane_shrinks_to_its_content() {
    let width = keyboard_pane_width(&KeyboardState::default(), Instant::now());

    assert!(width < KEYBOARD_PANE_MAX_WIDTH, "{width}");
}

/// tick ごとに Combo の引いた数が 1 桁から 2 桁へ増えても、pane の幅は変わらない。
#[test]
fn the_pane_keeps_its_width_while_the_combo_count_grows() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_velocity(now);
    state.cycle_velocity(now);
    state.cycle_modulation(now);
    state.cycle_modulation(now);
    for _ in 0..5 {
        state.cycle_pitch_bend(now);
    }
    state.toggle_cc_periodic(now); // vel2 × mod2 × PB4 × CC2 = 32 通り
    let first = keyboard_pane_width(&state, now);

    let mut drawn_counts = Vec::new();
    for tick in 1..=20u64 {
        let at = now + Duration::from_millis(250 * tick);
        let _ = state.poll_periodic(at);
        drawn_counts.extend(state.combo_progress().map(|(drawn, _)| drawn));
        assert_eq!(keyboard_pane_width(&state, at), first, "tick {tick}");
    }
    assert!(
        drawn_counts.contains(&9) && drawn_counts.contains(&10),
        "{drawn_counts:?}"
    );
}
