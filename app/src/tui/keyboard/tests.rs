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
fn press_tracks_last_chord_and_restarts_after_full_release() {
    let mut state = KeyboardState::default();
    // c e g の同時押しは和音として記憶される
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    assert!(state.press(KEYBOARD_NOTES[4]).is_some());
    assert_eq!(
        state.repeat_chords()[0]
            .iter()
            .map(|note| note.midi_note)
            .collect::<Vec<_>>(),
        vec![60, 64, 67]
    );
    // 全release後も和音は保持される
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[2]).is_some());
    assert!(state.release(KEYBOARD_NOTES[4]).is_some());
    assert_eq!(state.repeat_chords()[0].len(), 3);
    // 新しい単独押しで和音が置き換わる
    assert!(state.press(KEYBOARD_NOTES[1]).is_some());
    assert_eq!(
        state.repeat_chords()[0]
            .iter()
            .map(|note| note.midi_note)
            .collect::<Vec<_>>(),
        vec![62]
    );
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
    let now = std::time::Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert_eq!(state.cycle_modulation(now), [0xB0, 1, 127]);

    assert_eq!(
        state.take_reset_messages(),
        vec![[0x80, 60, 0], [0xB0, 1, 0]]
    );
    assert_eq!(state.modulation_mode(), ModulationMode::Off);
    assert!(state.take_reset_messages().is_empty());
}

#[test]
fn take_reset_messages_resets_pitch_bend_periodic_cc_and_note_repeat() {
    let mut state = KeyboardState::default();
    let now = std::time::Instant::now();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');
    assert_eq!(state.confirm_numeric_input(), None);

    state.cycle_velocity(now);
    state.cycle_velocity(now); // Periodic(velocity=100)
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic
    state.cycle_pitch_bend(now);
    state.cycle_pitch_bend(now);
    state.cycle_pitch_bend(now); // Min
    state.toggle_cc_periodic(now);
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(now); // 和音cをリトリガー中にする

    assert_eq!(
        state.take_reset_messages(),
        vec![
            [0x80, 60, 0], // held c
            [0x80, 60, 0], // repeat sounding c
            [0xB0, 1, 0],
            [0xE0, 0x00, 0x40], // pitch bend center
            [0xB0, 74, 0],
        ]
    );
    assert_eq!(state.velocity_mode(), VelocityMode::Normal);
    assert_eq!(state.velocity(), 100);
    assert_eq!(state.modulation_mode(), ModulationMode::Off);
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Idle);
    assert!(!state.cc_periodic_on());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
    assert!(state.take_reset_messages().is_empty());
    assert!(state
        .poll_periodic(now + Duration::from_secs(60))
        .is_empty());
}

#[test]
fn pitch_bend_message_encodes_14bit_value() {
    assert_eq!(pitch_bend(16383), [0xE0, 0x7F, 0x7F]);
    assert_eq!(pitch_bend(8192), [0xE0, 0x00, 0x40]);
    assert_eq!(pitch_bend(0), [0xE0, 0x00, 0x00]);
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
