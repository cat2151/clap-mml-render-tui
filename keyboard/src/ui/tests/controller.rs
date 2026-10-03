use super::*;
use cmrt_tui_core::theme::{MONOKAI_GRAY, MONOKAI_GREEN};

fn controller_text_of(state: &KeyboardState) -> Vec<String> {
    controller_status_lines(state)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}

/// 各行の選択肢と、それが今選ばれているか。
fn choices_of(state: &KeyboardState) -> Vec<Vec<(String, bool)>> {
    controller_status_lines(state)
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .filter(|span| matches!(span.style.fg, Some(MONOKAI_GREEN | MONOKAI_GRAY)))
                .map(|span| {
                    (
                        span.content.to_string(),
                        span.style.fg == Some(MONOKAI_GREEN),
                    )
                })
                .collect()
        })
        .collect()
}

fn selected_of(state: &KeyboardState) -> Vec<Option<usize>> {
    choices_of(state)
        .iter()
        .map(|choices| choices.iter().position(|(_, selected)| *selected))
        .collect()
}

#[test]
fn each_controller_has_its_own_row_listing_every_choice() {
    assert_eq!(
        controller_text_of(&KeyboardState::default()),
        [
            "vel: 100  127  cyc",
            "mod: off  on  cyc",
            "pb: +8191  0  -8192  0  cyc  0",
            "cc#(x): 1  Z: off  cyc",
        ]
    );
}

#[test]
fn only_the_current_choice_is_colored_by_default() {
    let state = KeyboardState::default();
    // pb は一度も押していない間はどれも選ばれていない。
    assert_eq!(selected_of(&state), [Some(0), Some(0), None, Some(0)]);
    for choices in choices_of(&state) {
        assert!(choices.iter().filter(|(_, selected)| *selected).count() <= 1);
    }
}

#[test]
fn periodic_modes_are_colored_and_show_combo_progress() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    state.cycle_velocity(now);
    state.cycle_velocity(now); // Periodic(velocity=100)
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic
    for _ in 0..5 {
        state.cycle_pitch_bend(now); // Periodicまで進める
    }
    state.toggle_cc_periodic(now);

    // vel2 × mod2 × PB4 × CC2 = 32通り。tick前なので消化数は0
    assert_eq!(
        controller_text_of(&state),
        [
            "vel: 100  127  cyc(100)",
            "mod: off  on  cyc",
            "pb: +8191  0  -8192  0  cyc  0",
            "cc#(x): 1  Z: off  cyc  Combo: 0/32",
        ]
    );
    assert_eq!(selected_of(&state), [Some(2), Some(2), Some(4), Some(1)]);
}

#[test]
fn velocity_and_modulation_selection_follows_each_press() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    for expected in [1, 2, 0] {
        state.cycle_velocity(now);
        state.cycle_modulation(now);
        assert_eq!(selected_of(&state)[..2], [Some(expected), Some(expected)]);
    }
}

#[test]
fn combo_progress_appears_only_with_periodic_digits() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    assert!(!controller_text_of(&state).concat().contains("Combo:"));
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic
    state.toggle_cc_periodic(now);
    assert!(controller_text_of(&state)[3].ends_with("Combo: 0/4"));
    let _ = state.poll_periodic(now + std::time::Duration::from_millis(250));
    assert!(controller_text_of(&state)[3].ends_with("Combo: 1/4"));
}

#[test]
fn pitch_bend_selection_walks_the_cycle_in_order() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    for expected in [0, 1, 2, 3, 4, 5, 0] {
        state.cycle_pitch_bend(now);
        assert_eq!(selected_of(&state)[2], Some(expected));
    }
}

#[test]
fn cc_cycle_selection_follows_the_toggle() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    state.toggle_cc_periodic(now);
    assert_eq!(selected_of(&state)[3], Some(1));
    state.toggle_cc_periodic(now);
    assert_eq!(selected_of(&state)[3], Some(0));
}
