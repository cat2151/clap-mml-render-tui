use super::*;

#[test]
fn note_mapping_is_c4_through_b4() {
    assert_eq!(
        KEYBOARD_NOTES
            .iter()
            .map(|note| (note.key, note.midi_note))
            .collect::<Vec<_>>(),
        vec![
            ('c', 60),
            ('d', 62),
            ('e', 64),
            ('f', 65),
            ('g', 67),
            ('a', 69),
            ('b', 71)
        ]
    );
}

#[test]
fn notes_remain_active_until_each_key_is_released() {
    let mut state = KeyboardState::default();
    assert_eq!(state.press(KEYBOARD_NOTES[0]), Some(vec![[0x90, 60, 100]]));
    assert_eq!(state.press(KEYBOARD_NOTES[1]), Some(vec![[0x90, 62, 100]]));
    assert_eq!(state.held(), &KEYBOARD_NOTES[..2]);
    assert_eq!(state.release(KEYBOARD_NOTES[0]), Some(vec![[0x80, 60, 0]]));
    assert_eq!(state.held(), &KEYBOARD_NOTES[1..2]);
    assert_eq!(state.release(KEYBOARD_NOTES[1]), Some(vec![[0x80, 62, 0]]));
    assert!(state.held().is_empty());
}

#[test]
fn duplicate_press_and_unknown_release_send_nothing() {
    let mut state = KeyboardState::default();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert_eq!(state.press(KEYBOARD_NOTES[0]), None);
    assert_eq!(state.release(KEYBOARD_NOTES[1]), None);
    assert_eq!(state.held(), &KEYBOARD_NOTES[..1]);
}

#[test]
fn take_reset_messages_stops_every_held_note_and_clears_state() {
    let mut state = KeyboardState::default();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    assert!(state.press(KEYBOARD_NOTES[4]).is_some());

    assert_eq!(
        state.take_reset_messages(),
        vec![[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0]]
    );
    assert!(state.held().is_empty());
}

#[test]
fn take_reset_messages_appends_modulation_reset_when_modulation_is_on() {
    let mut state = KeyboardState::default();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert_eq!(state.toggle_modulation(), [0xB0, 1, 127]);

    assert_eq!(
        state.take_reset_messages(),
        vec![[0x80, 60, 0], [0xB0, 1, 0]]
    );
    assert!(!state.modulation_on());
    assert!(state.take_reset_messages().is_empty());
}

#[test]
fn velocity_toggles_between_100_and_127_and_applies_to_note_on() {
    let mut state = KeyboardState::default();
    assert_eq!(state.velocity(), 100);
    assert_eq!(state.toggle_velocity(), 127);
    assert_eq!(state.press(KEYBOARD_NOTES[0]), Some(vec![[0x90, 60, 127]]));
    assert_eq!(state.toggle_velocity(), 100);
    assert_eq!(state.press(KEYBOARD_NOTES[1]), Some(vec![[0x90, 62, 100]]));
}

#[test]
fn modulation_toggles_cc1_between_127_and_0() {
    let mut state = KeyboardState::default();
    assert!(!state.modulation_on());
    assert_eq!(state.toggle_modulation(), [0xB0, 1, 127]);
    assert!(state.modulation_on());
    assert_eq!(state.toggle_modulation(), [0xB0, 1, 0]);
    assert!(!state.modulation_on());
}

#[test]
fn cc_number_input_updates_number_without_sending() {
    let mut state = KeyboardState::default();
    assert_eq!(state.cc_number(), 1);

    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');
    assert_eq!(state.numeric_input().map(NumericInput::buffer), Some("74"));

    assert_eq!(state.confirm_numeric_input(), None);
    assert_eq!(state.cc_number(), 74);
    assert!(state.numeric_input().is_none());
}

#[test]
fn cc_value_input_sends_to_current_cc_number_and_defaults_to_cc1() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcValue);
    state.numeric_input_push('9');
    state.numeric_input_push('9');
    assert_eq!(state.confirm_numeric_input(), Some([0xB0, 1, 99]));

    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');
    assert_eq!(state.confirm_numeric_input(), None);

    state.begin_numeric_input(NumericInputTarget::CcValue);
    state.numeric_input_push('0');
    assert_eq!(state.confirm_numeric_input(), Some([0xB0, 74, 0]));
}

#[test]
fn numeric_input_clamps_to_127_and_limits_to_three_digits() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('9');
    state.numeric_input_push('9');
    state.numeric_input_push('9');
    state.numeric_input_push('9');
    assert_eq!(state.numeric_input().map(NumericInput::buffer), Some("999"));

    assert_eq!(state.confirm_numeric_input(), None);
    assert_eq!(state.cc_number(), 127);
}

#[test]
fn numeric_input_supports_backspace_cancel_and_empty_confirm() {
    let mut state = KeyboardState::default();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_backspace();
    assert_eq!(state.numeric_input().map(NumericInput::buffer), Some(""));

    // 空のままEnterは何も変更せず入力モードを抜ける
    assert_eq!(state.confirm_numeric_input(), None);
    assert_eq!(state.cc_number(), 1);
    assert!(state.numeric_input().is_none());

    state.begin_numeric_input(NumericInputTarget::CcValue);
    state.numeric_input_push('5');
    state.cancel_numeric_input();
    assert!(state.numeric_input().is_none());
}

#[test]
fn keyboard_state_keeps_non_blank_patch() {
    let state = KeyboardState::new(Some("patches_factory/Keys/Piano.fxp".to_string()));

    assert_eq!(state.patch(), Some("patches_factory/Keys/Piano.fxp"));
    assert_eq!(KeyboardState::new(Some("  ".to_string())).patch(), None);
}

#[test]
fn buffer_multiplier_defaults_to_x4_and_cycles_x8_x1_x2_x4() {
    let mut state = KeyboardState::default();
    assert_eq!(state.buffer_multiplier(), 4);
    assert_eq!(state.cycle_buffer_multiplier(), 8);
    assert_eq!(state.cycle_buffer_multiplier(), 1);
    assert_eq!(state.cycle_buffer_multiplier(), 2);
    assert_eq!(state.cycle_buffer_multiplier(), 4);
}

#[test]
fn keyboard_state_defaults_to_shared_memory() {
    assert_eq!(
        KeyboardState::default().transport(),
        KeyboardTransport::SharedMemory
    );
}
