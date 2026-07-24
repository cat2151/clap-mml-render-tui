use super::*;

fn press_and_release(state: &mut KeyboardState, notes: &[KeyboardNote]) {
    for note in notes {
        assert!(state.press(*note).is_some());
    }
    for note in notes {
        assert!(state.release(*note).is_some());
    }
}

fn enter_arp(state: &mut KeyboardState, now: Instant) -> Vec<[u8; 3]> {
    let _ = state.cycle_note_playback(now);
    state.cycle_note_playback(now)
}

#[test]
fn note_playback_cycles_off_repeat_arp_auto_off_with_unknown_fallback() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);

    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x90, 60, 100], [0x90, 64, 100]]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Repeat);
    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x80, 60, 0], [0x80, 64, 0], [0x90, 60, 100]]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Arp);
    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x80, 60, 0], [0x90, 60, 100], [0x90, 64, 100]]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Auto);
    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x80, 60, 0], [0x80, 64, 0]]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
}

#[test]
fn auto_uses_arp_for_a_mono_detection() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Mono);
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut state, now);

    assert!(state.cycle_note_playback(now).is_empty());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Auto);
    assert!(state.note_playback_uses_arp());
    assert_eq!(
        state.poll_periodic(now + Duration::from_millis(250)),
        vec![[0x80, 60, 0], [0x90, 64, 100]]
    );
}

#[test]
fn auto_uses_repeat_for_a_poly_detection() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Poly);
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut state, now);

    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x80, 60, 0], [0x90, 60, 100], [0x90, 64, 100]]
    );
    assert!(!state.note_playback_uses_arp());
    for tick in 1..8 {
        assert!(state
            .poll_periodic(now + PERIODIC_INTERVAL * tick)
            .is_empty());
    }
    assert_eq!(
        state.poll_periodic(now + PERIODIC_INTERVAL * 8),
        vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x90, 60, 100],
            [0x90, 64, 100],
        ]
    );
}

#[test]
fn note_playback_stays_off_without_a_chord() {
    let mut state = KeyboardState::default();
    assert!(state.cycle_note_playback(Instant::now()).is_empty());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
}

#[test]
fn replacing_target_while_off_only_updates_the_stored_notes() {
    let mut state = KeyboardState::default();

    assert!(state
        .replace_repeat_chords(vec![vec![67, 60, 67]], Instant::now(), true)
        .is_empty());
    assert_eq!(
        state.repeat_chords()[0]
            .iter()
            .map(|note| note.midi_note)
            .collect::<Vec<_>>(),
        vec![67, 60]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
}

#[test]
fn replacing_target_deduplicates_within_each_chord_but_keeps_repeated_chords() {
    let mut state = KeyboardState::default();

    state.replace_repeat_chords(
        vec![vec![67, 60, 67], vec![67, 60, 67], vec![], vec![64]],
        Instant::now(),
        false,
    );

    assert_eq!(
        state
            .repeat_chords()
            .iter()
            .map(|chord| chord.iter().map(|note| note.midi_note).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![67, 60], vec![67, 60], vec![64]]
    );
}

#[test]
fn replacing_target_restarts_repeat_immediately() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = state.cycle_note_playback(now);

    assert_eq!(
        state.replace_repeat_chords(vec![vec![67, 71]], now, true),
        vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x90, 67, 100],
            [0x90, 71, 100],
        ]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Repeat);
}

#[test]
fn replacing_target_restarts_arp_from_the_new_lowest_note() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut state, now);

    assert_eq!(
        state.replace_repeat_chords(vec![vec![71, 65]], now, true),
        vec![[0x80, 60, 0], [0x90, 65, 100]]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Arp);
}

#[test]
fn replacing_target_restarts_auto_using_the_detected_voicing() {
    let now = Instant::now();
    let mut mono = KeyboardState::default();
    mono.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Mono);
    press_and_release(&mut mono, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut mono, now);
    assert!(mono.cycle_note_playback(now).is_empty());
    assert_eq!(
        mono.replace_repeat_chords(vec![vec![71, 65]], now, true),
        vec![[0x80, 60, 0], [0x90, 65, 100]]
    );

    let mut poly = KeyboardState::default();
    poly.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Poly);
    press_and_release(&mut poly, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut poly, now);
    let _ = poly.cycle_note_playback(now);
    assert_eq!(
        poly.replace_repeat_chords(vec![vec![71, 65]], now, true),
        vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x90, 71, 100],
            [0x90, 65, 100],
        ]
    );
}

#[test]
fn replacing_target_while_unready_is_used_by_the_ready_refresh() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = state.cycle_note_playback(now);
    assert_eq!(
        state.take_note_off_messages(),
        vec![[0x80, 60, 0], [0x80, 64, 0]]
    );

    assert!(state
        .replace_repeat_chords(vec![vec![67, 71]], now, false)
        .is_empty());
    assert_eq!(
        state.take_pending_refresh_messages(now),
        vec![[0x90, 67, 100], [0x90, 71, 100]]
    );
}

#[test]
fn arp_sorts_notes_and_plays_two_octaves_every_250ms() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(
        &mut state,
        &[
            KEYBOARD_NOTES[4],
            KEYBOARD_NOTES[0],
            KEYBOARD_NOTES[6],
            KEYBOARD_NOTES[2],
        ],
    );

    assert_eq!(
        enter_arp(&mut state, now),
        vec![
            [0x80, 67, 0],
            [0x80, 60, 0],
            [0x80, 71, 0],
            [0x80, 64, 0],
            [0x90, 60, 100],
        ]
    );
    assert!(state
        .poll_periodic(now + Duration::from_millis(249))
        .is_empty());

    let expected_notes = [64, 67, 71, 72, 76, 79, 83, 60];
    let mut previous = 60;
    for (step, note) in expected_notes.into_iter().enumerate() {
        assert_eq!(
            state.poll_periodic(now + Duration::from_millis(250 * (step as u64 + 1))),
            vec![[0x80, previous, 0], [0x90, note, 100]]
        );
        previous = note;
    }
}

#[test]
fn arp_finishes_each_chord_sequence_before_advancing_the_progression() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.replace_repeat_chords(vec![vec![64, 60], vec![69, 65]], now, false);

    assert_eq!(
        enter_arp(&mut state, now),
        vec![[0x80, 64, 0], [0x80, 60, 0], [0x90, 60, 100]]
    );
    let expected_notes = [64, 72, 76, 65, 69, 77, 81, 60];
    let mut previous = 60;
    for (step, note) in expected_notes.into_iter().enumerate() {
        assert_eq!(
            state.poll_periodic(now + Duration::from_millis(250 * (step as u64 + 1))),
            vec![[0x80, previous, 0], [0x90, note, 100]]
        );
        previous = note;
    }
}

#[test]
fn arp_uses_current_velocity_for_each_attack() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0]]);
    let _ = enter_arp(&mut state, now);
    state.cycle_velocity(now);

    assert_eq!(
        state.poll_periodic(now + Duration::from_millis(250)),
        vec![[0x80, 60, 0], [0x90, 72, 127]]
    );
}

#[test]
fn arp_skips_missed_steps_after_a_long_stall() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut state, now);

    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(10)),
        vec![[0x80, 60, 0], [0x90, 64, 100]]
    );
    assert!(state
        .poll_periodic(now + Duration::from_millis(10_249))
        .is_empty());
    assert_eq!(
        state.poll_periodic(now + Duration::from_millis(10_250)),
        vec![[0x80, 64, 0], [0x90, 72, 100]]
    );
}

#[test]
fn new_chord_restarts_arp_from_its_lowest_note_on_next_tick() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut state, now);
    press_and_release(&mut state, &[KEYBOARD_NOTES[4], KEYBOARD_NOTES[1]]);

    assert_eq!(
        state.poll_periodic(now + Duration::from_millis(250)),
        vec![[0x80, 60, 0], [0x90, 62, 100]]
    );
}

#[test]
fn arp_and_periodic_controller_share_each_250ms_tick() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0]]);
    state.cycle_modulation(now);
    state.cycle_modulation(now);
    let _ = enter_arp(&mut state, now);

    let messages = state.poll_periodic(now + Duration::from_millis(250));
    assert_eq!(messages.len(), 3);
    assert_eq!(&messages[0][..2], &[0xB0, 1]);
    assert_eq!(messages[1], [0x80, 60, 0]);
    assert_eq!(messages[2], [0x90, 72, 100]);
}

#[test]
fn arp_refresh_restarts_from_first_note_with_a_fresh_deadline() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0], KEYBOARD_NOTES[2]]);
    let _ = enter_arp(&mut state, now);
    assert_eq!(state.take_note_off_messages(), vec![[0x80, 60, 0]]);

    let ready_at = now + Duration::from_secs(2);
    assert_eq!(
        state.take_pending_refresh_messages(ready_at),
        vec![[0x90, 60, 100]]
    );
    assert!(state.poll_periodic(ready_at).is_empty());
    assert_eq!(
        state.poll_periodic(ready_at + Duration::from_millis(250)),
        vec![[0x80, 60, 0], [0x90, 64, 100]]
    );
}

#[test]
fn reset_stops_arp_and_clears_its_deadline() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    press_and_release(&mut state, &[KEYBOARD_NOTES[0]]);
    let _ = enter_arp(&mut state, now);

    assert_eq!(state.take_reset_messages(), vec![[0x80, 60, 0]]);
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
    assert!(state
        .poll_periodic(now + Duration::from_secs(60))
        .is_empty());
}
