use super::*;
use crate::tui::keyboard::KeyboardState;
use ratatui::{backend::TestBackend, Terminal};

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_overlay(phase: KeyboardConnectionPhase) -> String {
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| draw_connection_overlay(&phase, f, f.area()))
        .unwrap();
    buffer_to_string(&terminal)
}

fn render_numeric_overlay(state: &KeyboardState) -> String {
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| draw_numeric_input_overlay(state.numeric_input(), state.cc_number(), f, f.area()))
        .unwrap();
    buffer_to_string(&terminal)
}

#[test]
fn active_notes_text_lists_every_held_note_in_press_order() {
    let mut state = KeyboardState::default();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    assert!(state.press(KEYBOARD_NOTES[4]).is_some());

    assert_eq!(active_notes_text(&state), "Active: C4 E4 G4");
}

#[test]
fn active_notes_text_shows_dash_when_no_notes_are_held() {
    assert_eq!(active_notes_text(&KeyboardState::default()), "Active: -");
}

#[test]
fn controller_status_text_shows_defaults() {
    assert_eq!(
        controller_status_text(&KeyboardState::default()),
        "Vel: 100  Mod: OFF  PB: -  CC#: 1"
    );
}

#[test]
fn controller_status_text_shows_periodic_modes() {
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
        controller_status_text(&state),
        "Vel: cyc(100)  Mod: CYC  PB: CYC  CC#: 1 cyc  Combo: 0/32"
    );
}

#[test]
fn controller_status_text_shows_combo_progress_only_with_periodic_digits() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    assert!(!controller_status_text(&state).contains("Combo:"));
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic
    state.toggle_cc_periodic(now);
    assert!(controller_status_text(&state).contains("Combo: 0/4"));
    let _ = state.poll_periodic(now + std::time::Duration::from_secs(1));
    assert!(controller_status_text(&state).contains("Combo: 1/4"));
}

#[test]
fn controller_status_text_shows_fixed_pitch_bend_values() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: +8191"));
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: 0"));
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: -8192"));
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: 0"));
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: CYC"));
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: 0"));
    state.cycle_pitch_bend(now);
    assert!(controller_status_text(&state).contains("PB: +8191"));
}

#[test]
fn repeat_status_text_shows_dash_when_off_and_chord_when_on() {
    let mut state = KeyboardState::default();
    assert_eq!(repeat_status_text(&state), "Repeat: -");

    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    assert!(state.press(KEYBOARD_NOTES[4]).is_some());
    let _ = state.toggle_note_repeat(std::time::Instant::now());
    assert_eq!(repeat_status_text(&state), "Repeat: C4 E4 G4");

    let _ = state.toggle_note_repeat(std::time::Instant::now());
    assert_eq!(repeat_status_text(&state), "Repeat: -");
}

#[test]
fn send_duration_uses_microseconds_then_milliseconds() {
    assert_eq!(
        format_send_duration(std::time::Duration::from_micros(42)),
        "42 us"
    );
    assert_eq!(
        format_send_duration(std::time::Duration::from_micros(12_345)),
        "12.3 ms"
    );
}

#[test]
fn connecting_overlay_explains_that_notes_are_unavailable() {
    let screen = render_overlay(KeyboardConnectionPhase::Connecting);

    assert!(screen.contains("connecting..."));
    assert!(screen.contains("notes unavailable until ready"));
}

#[test]
fn error_overlay_shows_retry_navigation() {
    let screen = render_overlay(KeyboardConnectionPhase::Error("server failed".to_string()));

    assert!(screen.contains("server error: server failed"));
    assert!(screen.contains("r:retry"));
}

#[test]
fn patch_setting_overlay_remains_until_patch_is_ready() {
    let screen = render_overlay(KeyboardConnectionPhase::PatchSetting);

    assert!(screen.contains("patch setting..."));
    assert!(screen.contains("notes unavailable until ready"));
}

#[test]
fn cc_number_input_overlay_shows_typed_digits_and_key_help() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');

    let screen = render_numeric_overlay(&state);
    // 全角文字はセル埋めの空白を挟んで描画されるため、空白を除去して比較する
    let normalized = screen.replace(' ', "");
    assert!(screen.contains("CC number"));
    assert!(normalized.contains("CC番号を入力:74_"));
    assert!(normalized.contains("Enter:確定"));
}

#[test]
fn cc_value_input_overlay_names_the_target_cc_number() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcValue);

    let screen = render_numeric_overlay(&state);
    let normalized = screen.replace(' ', "");
    assert!(screen.contains("CC value"));
    assert!(normalized.contains("CC値を入力(CC#1へ送信):_"));
}

#[test]
fn no_numeric_input_draws_no_overlay() {
    assert!(render_numeric_overlay(&KeyboardState::default())
        .chars()
        .all(char::is_whitespace));
}

#[test]
fn ready_connection_does_not_draw_an_overlay() {
    assert!(render_overlay(KeyboardConnectionPhase::Ready)
        .chars()
        .all(char::is_whitespace));
}
