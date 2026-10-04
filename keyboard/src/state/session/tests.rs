use super::*;

fn restored(mode: NotePlaybackMode, repeat_chords: Vec<Vec<u8>>) -> KeyboardState {
    KeyboardState::from_session(KeyboardSessionState {
        note_playback_mode: mode,
        repeat_chords,
        ..KeyboardSessionState::default()
    })
}

fn chord_notes(state: &KeyboardState) -> Vec<Vec<u8>> {
    state
        .repeat_chords()
        .iter()
        .map(|chord| chord.iter().map(|note| note.midi_note).collect())
        .collect()
}

#[test]
fn a_saved_target_and_mode_are_restored() {
    let state = restored(NotePlaybackMode::Repeat, vec![vec![62, 69]]);

    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Repeat);
    assert_eq!(chord_notes(&state), vec![vec![62, 69]]);
}

#[test]
fn a_saved_progression_is_restored_in_order() {
    let progression = vec![vec![60, 64, 67], vec![65, 69, 72], vec![67, 71, 74]];
    let state = restored(NotePlaybackMode::Arp, progression.clone());

    assert_eq!(chord_notes(&state), progression);
}

#[test]
fn restored_target_goes_through_the_same_cleanup_as_mml_targets() {
    let state = restored(
        NotePlaybackMode::Repeat,
        vec![vec![67, 60, 67], vec![], vec![200, 64]],
    );

    assert_eq!(chord_notes(&state), vec![vec![67, 60], vec![64]]);
}

#[test]
fn an_empty_saved_target_falls_back_to_the_mode_default() {
    for (mode, expected) in [
        (NotePlaybackMode::Auto, vec![vec![60, 65, 67]]),
        (NotePlaybackMode::Arp, vec![vec![60, 65, 67]]),
        (NotePlaybackMode::Repeat, vec![vec![60]]),
        (NotePlaybackMode::Off, Vec::new()),
    ] {
        let state = restored(mode, Vec::new());
        assert_eq!(state.note_playback_mode(), mode);
        assert_eq!(chord_notes(&state), expected, "{mode:?}");
    }
}

#[test]
fn a_restored_repeat_starts_on_the_first_ready_refresh() {
    let mut state = restored(NotePlaybackMode::Repeat, vec![vec![62, 69]]);
    let now = Instant::now();

    assert_eq!(
        state.take_pending_refresh_messages(now),
        vec![[0x90, 62, 100], [0x90, 69, 100]]
    );
    assert!(state.take_pending_refresh_messages(now).is_empty());
}

#[test]
fn a_restored_off_plays_nothing_on_ready() {
    let mut state = restored(NotePlaybackMode::Off, vec![vec![62, 69]]);
    let now = Instant::now();

    assert!(state.take_pending_refresh_messages(now).is_empty());
    assert!(state
        .poll_periodic(now + Duration::from_secs(10))
        .is_empty());
}

#[test]
fn t_does_nothing_right_after_restoring_off_without_a_target() {
    let mut state = restored(NotePlaybackMode::Off, Vec::new());

    assert!(state.cycle_note_playback(Instant::now()).is_empty());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
}

#[test]
fn session_state_writes_the_mode_target_and_given_mml() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.replace_repeat_chords(vec![vec![60, 64], vec![65]], now, false);
    let _ = state.cycle_note_playback(now);

    let session = state.session_state("c;e f".to_string());
    assert_eq!(session.note_playback_mode, NotePlaybackMode::Auto);
    assert_eq!(session.repeat_chords, vec![vec![60, 64], vec![65]]);
    assert_eq!(session.mml, "c;e f");
}

#[test]
fn restarting_with_a_patch_keeps_the_mode_and_target() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[1]).is_some());
    assert!(state.release(KEYBOARD_NOTES[1]).is_some());
    let _ = state.cycle_note_playback(now);
    let _ = state.cycle_note_playback(now);

    let restarted = state.restart_with_patch(Some("b.fxp".to_string()));
    assert_eq!(restarted.patch(), Some("b.fxp"));
    assert_eq!(restarted.note_playback_mode(), NotePlaybackMode::Repeat);
    assert_eq!(chord_notes(&restarted), vec![vec![62]]);
}

#[test]
fn leaving_the_screen_silences_but_keeps_the_mode_for_the_next_ready() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(now);

    assert_eq!(state.take_leave_messages(), vec![[0x80, 60, 0]]);
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Auto);
    assert_eq!(
        state.take_pending_refresh_messages(now),
        vec![[0x90, 60, 100]]
    );
}

#[test]
fn a_saved_patch_filter_is_restored_and_saved_again() {
    let state = KeyboardState::from_session(KeyboardSessionState {
        patch_filter: "lead -plugin:surgext".to_string(),
        ..KeyboardSessionState::default()
    });

    assert_eq!(state.patch_catalog.filter(), "lead -plugin:surgext");
    assert_eq!(
        state.session_state(String::new()).patch_filter,
        "lead -plugin:surgext"
    );
}

#[test]
fn a_saved_patch_filter_that_does_not_compile_is_dropped() {
    let state = KeyboardState::from_session(KeyboardSessionState {
        patch_filter: "lead(".to_string(),
        ..KeyboardSessionState::default()
    });

    assert_eq!(state.patch_catalog.filter(), "");
}

fn every_controller_changed() -> KeyboardState {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    let _ = state.cycle_velocity(now);
    let _ = state.cycle_modulation(now);
    let _ = state.cycle_pitch_bend(now);
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    let _ = state.confirm_numeric_input();
    let _ = state.toggle_cc_periodic(now);
    state
}

#[test]
fn controller_selections_survive_leaving_saving_and_restoring() {
    let mut state = every_controller_changed();

    // 画面を出るときは消音するが、選択は残す
    let leave = state.take_leave_messages();
    assert!(leave.contains(&[0xB0, 1, 0]), "{leave:?}");
    assert!(leave.contains(&[0xE0, 0x00, 0x40]), "{leave:?}");
    assert!(leave.contains(&[0xB0, 7, 0]), "{leave:?}");

    let restored = KeyboardState::from_session(state.session_state(String::new()));
    assert_eq!(restored.velocity_mode(), VelocityMode::Accent);
    assert_eq!(restored.velocity(), 127);
    assert_eq!(restored.modulation_mode(), ModulationMode::On);
    assert_eq!(restored.pitch_bend_mode(), PitchBendMode::Max);
    assert_eq!(restored.cc_number(), 7);
    assert!(restored.cc_periodic_on());
}

#[test]
fn restored_controllers_are_sent_and_cycled_on_the_first_ready() {
    let mut state =
        KeyboardState::from_session(every_controller_changed().session_state(String::new()));
    let now = Instant::now();

    let refresh = state.take_pending_refresh_messages(now);
    assert!(refresh.contains(&[0xB0, 1, 127]), "{refresh:?}");
    assert!(refresh.contains(&[0xE0, 0x7F, 0x7F]), "{refresh:?}");
    assert!(refresh.contains(&[0xB0, 7, 127]), "{refresh:?}");
    // Z の周期送信が、t が off でもクロックごと動き出す
    assert_eq!(state.periodic_anchor(), Some(now));
    let tick = state.poll_periodic(now + PERIODIC_INTERVAL);
    assert!(
        tick.iter()
            .any(|message| message[0] == 0xB0 && message[1] == 7),
        "{tick:?}"
    );
}

#[test]
fn a_restored_periodic_velocity_starts_from_the_first_value() {
    let state = KeyboardState::from_session(KeyboardSessionState {
        controllers: KeyboardControllerState {
            velocity: VelocityMode::Periodic,
            ..KeyboardControllerState::default()
        },
        ..KeyboardSessionState::default()
    });

    assert_eq!(state.velocity_mode(), VelocityMode::Periodic);
    assert_eq!(state.velocity(), 100);
}

#[test]
fn untouched_controllers_send_nothing_on_the_first_ready() {
    let mut state = KeyboardState::from_session(KeyboardSessionState::default());

    assert!(state
        .take_pending_refresh_messages(Instant::now())
        .is_empty());
    assert_eq!(state.periodic_anchor(), None);
}
