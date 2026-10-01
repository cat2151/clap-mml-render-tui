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
