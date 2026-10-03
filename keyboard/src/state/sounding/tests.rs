use std::time::{Duration, Instant};

use crate::periodic_timeline::LOOKAHEAD;
use crate::state::{KeyboardState, NotePlaybackMode, PlaybackNote, KEYBOARD_NOTES};

const TICK: Duration = Duration::from_millis(250);
const MS: Duration = Duration::from_millis(1);

fn chord_state(chords: Vec<Vec<u8>>, now: Instant) -> KeyboardState {
    let mut state = KeyboardState::default();
    let _ = state.replace_repeat_chords(chords, now, false);
    state
}

fn enter_arp(state: &mut KeyboardState, now: Instant) {
    let _ = state.cycle_note_playback(now);
    let _ = state.cycle_note_playback(now);
    let _ = state.cycle_note_playback(now);
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Arp);
}

fn enter_repeat(state: &mut KeyboardState, now: Instant) {
    let _ = state.cycle_note_playback(now);
    let _ = state.cycle_note_playback(now);
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Repeat);
}

/// (和音 index, arp 列の index, 音) を返す。
fn arp_at(state: &KeyboardState, now: Instant) -> (usize, usize, u8) {
    let position = state.sounding_position(now).expect("arp is sounding");
    let step = position.arp.expect("arp step");
    (position.chord_index, step.index, step.note.midi_note)
}

/// pump と同じく LOOKAHEAD 先までの tick を 1 つ進め、その deadline を返す。
fn pump(state: &mut KeyboardState, now: Instant) -> Instant {
    state
        .poll_periodic_tick(now + LOOKAHEAD)
        .expect("a tick is due")
        .at
}

#[test]
fn arp_start_sounds_the_lowest_note_immediately() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![67, 60]], t0);
    assert_eq!(state.sounding_position(t0), None);

    enter_arp(&mut state, t0);

    assert_eq!(arp_at(&state, t0), (0, 0, 60));
}

#[test]
fn a_ticked_arp_note_is_shown_only_from_its_deadline() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 67]], t0);
    enter_arp(&mut state, t0);

    let deadline = pump(&mut state, t0 + TICK - LOOKAHEAD);

    assert_eq!(deadline, t0 + TICK);
    assert_eq!(arp_at(&state, deadline - MS), (0, 0, 60));
    assert_eq!(arp_at(&state, deadline), (0, 1, 67));
}

#[test]
fn arp_wraps_after_the_upper_octave_and_advances_the_progression() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 67], vec![62, 65]], t0);
    enter_arp(&mut state, t0);

    let mut seen = vec![arp_at(&state, t0)];
    for step in 1..=5u32 {
        let deadline = pump(&mut state, t0 + TICK * step - LOOKAHEAD);
        seen.push(arp_at(&state, deadline));
    }

    assert_eq!(
        seen,
        vec![
            (0, 0, 60),
            (0, 1, 67),
            (0, 2, 72),
            (0, 3, 79),
            (1, 0, 62),
            (1, 1, 65),
        ]
    );
}

#[test]
fn a_single_chord_arp_returns_to_its_lowest_note() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 67]], t0);
    enter_arp(&mut state, t0);

    let mut deadline = t0;
    for step in 1..=4u32 {
        deadline = pump(&mut state, t0 + TICK * step - LOOKAHEAD);
    }

    assert_eq!(arp_at(&state, deadline - MS), (0, 3, 79));
    assert_eq!(arp_at(&state, deadline), (0, 0, 60));
}

#[test]
fn repeat_moves_to_the_next_chord_at_the_retrigger_deadline() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 64, 67], vec![62, 65]], t0);
    enter_repeat(&mut state, t0);
    let expect = |chord_index| {
        Some(crate::SoundingPosition {
            chord_index,
            arp: None,
        })
    };
    assert_eq!(state.sounding_position(t0), expect(0));

    let mut deadline = t0;
    for step in 1..=8u32 {
        deadline = pump(&mut state, t0 + TICK * step - LOOKAHEAD);
    }

    assert_eq!(state.sounding_position(deadline - MS), expect(0));
    assert_eq!(state.sounding_position(deadline), expect(1));
}

#[test]
fn off_and_a_new_progression_clear_the_position() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 67]], t0);
    enter_arp(&mut state, t0);
    let _ = state.cycle_note_playback(t0 + MS);
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
    assert_eq!(state.sounding_position(t0 + MS), None);

    enter_arp(&mut state, t0);
    let _ = state.replace_repeat_chords(vec![vec![62]], t0 + MS, false);
    assert_eq!(state.sounding_position(t0 + MS), None);

    let _ = state.replace_repeat_chords(vec![vec![65, 62]], t0 + MS * 2, true);
    assert_eq!(arp_at(&state, t0 + MS * 2), (0, 0, 62));
}

#[test]
fn reset_and_patch_switch_clear_the_position() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 67]], t0);
    enter_arp(&mut state, t0);
    let _ = state.take_note_off_messages();
    assert_eq!(state.sounding_position(t0), None);

    // Ready 復帰の再送は、その時刻から先頭の音を鳴らす
    let _ = state.take_pending_refresh_messages(t0 + MS);
    assert_eq!(arp_at(&state, t0 + MS), (0, 0, 60));

    let _ = state.take_reset_messages();
    assert_eq!(state.sounding_position(t0 + MS), None);
}

#[test]
fn a_manual_chord_reports_its_arp_from_the_played_notes() {
    let t0 = Instant::now();
    let mut state = KeyboardState::default();
    for note in [KEYBOARD_NOTES[4], KEYBOARD_NOTES[0]] {
        assert!(state.press(note).is_some());
    }
    for note in [KEYBOARD_NOTES[4], KEYBOARD_NOTES[0]] {
        assert!(state.release(note).is_some());
    }
    enter_arp(&mut state, t0);

    let position = state.sounding_position(t0).expect("arp is sounding");
    assert_eq!(
        position.arp.map(|step| step.note),
        Some(PlaybackNote { midi_note: 60 })
    );
}
