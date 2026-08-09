use std::time::Instant;

use super::*;
use crate::ChordPlayback;

fn c_major() -> ChordPlayback {
    ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap()
}

fn primary(instance: usize) -> LaneAddress {
    LaneAddress::new(instance, 0)
}

#[test]
fn note_edits_are_idempotent_and_reject_invalid_addresses() {
    let mut state = GridState::with_instance_count(3);
    let address = LaneAddress::new(2, 2);
    assert!(state.draw_note_span(address, 3, 6));
    assert!(!state.draw_note_span(address, 3, 6));
    assert!(!state.draw_note_span(LaneAddress::new(3, 0), 3, 6));
    assert!(!state.draw_note_span(address, GRID_STEPS, GRID_STEPS));
    assert_eq!(state.lane(address).unwrap().pattern.attack_len(3), Some(4));
    assert!(state.erase_note_at(address, 5));
    assert!(!state.erase_note_at(address, 5));
}

#[test]
fn instance_patch_changes_once_for_all_of_its_lanes() {
    let mut state = GridState::with_instance_count(3);
    state.instances_mut()[2].patch = Some("Keys/Old.fxp".to_string());
    assert!(state.set_instance_patch(2, "Keys/New.fxp".to_string()));
    assert!(!state.set_instance_patch(2, "Keys/New.fxp".to_string()));
    assert!(!state.set_instance_patch(3, "Keys/Invalid.fxp".to_string()));
    assert_eq!(state.instances()[2].patch.as_deref(), Some("Keys/New.fxp"));
    assert_eq!(state.instances()[2].lanes.len(), 4);
}

#[test]
fn note_edits_immediately_refresh_the_velocity_display_mask() {
    let mut state = GridState::with_instance_count(1);
    let address = primary(0);
    assert_eq!(state.velocity_display()[0][3], None);
    assert!(state.draw_note_span(address, 3, 5));
    assert!(state.velocity_display()[0][3].is_some());
    assert_eq!(state.velocity_display()[0][4], None, "Tie has no velocity");
    assert!(state.erase_note_at(address, 4));
    assert_eq!(state.velocity_display()[0][3], None);
}

#[test]
fn edits_affect_the_first_step_that_has_not_already_been_scheduled() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(1);
    state.start(now);
    let _ = state.poll_steps(now, crate::LOOKAHEAD);
    state.draw_note_span(primary(0), 1, 1);
    state.draw_note_span(primary(0), 3, 3);
    let next = state.poll_steps(now + crate::step_offset(2), crate::LOOKAHEAD);
    let note_ons = next
        .iter()
        .filter(|message| message.message[0] == 0x90)
        .collect::<Vec<_>>();
    assert_eq!(note_ons.len(), 1);
    assert_eq!(note_ons[0].message[1], 60);
}

#[test]
fn chord_summary_cannot_be_edited_but_the_triad_octave_voice_pattern_can() {
    let mut state = GridState::with_instance_count(3);
    state.set_chord(Some(c_major()), Instant::now());
    assert!(!state.draw_note_span(primary(CHORD_ROW), 3, 5));
    let octave_voice = LaneAddress::new(2, 3);
    assert!(state.draw_note_span(octave_voice, 3, 5));
    assert_eq!(state.resolved_note(octave_voice), Some(72));
    assert!(!state.move_lane_pitch(octave_voice, PitchDirection::Up));
}

#[test]
fn pitch_moves_by_semitone_without_a_chord() {
    let mut state = GridState::with_instance_count(1);
    state.lane_mut(primary(0)).unwrap().base_note = 60;
    assert!(state.move_lane_pitch(primary(0), PitchDirection::Up));
    assert_eq!(state.lane(primary(0)).unwrap().base_note, 61);
    state.lane_mut(primary(0)).unwrap().base_note = 127;
    assert!(!state.move_lane_pitch(primary(0), PitchDirection::Up));
}

#[test]
fn single_lane_moves_between_chord_tones_but_chord_voices_are_locked() {
    let mut state = GridState::with_instance_count(4);
    state.lane_mut(primary(3)).unwrap().base_note = 60;
    state.set_chord(Some(c_major()), Instant::now());
    assert!(state.move_lane_pitch(primary(3), PitchDirection::Up));
    assert_eq!(state.resolved_note(primary(3)), Some(64));
    assert!(!state.move_lane_pitch(primary(2), PitchDirection::Up));
    // bass 行の音高もコードから導出するので、保存値は動かせない。
    assert!(!state.move_lane_pitch(primary(BASS_ROW), PitchDirection::Up));
}

#[test]
fn chord_voicing_rotation_accumulates_in_both_directions() {
    let mut state = GridState::with_instance_count(3);
    state.set_chord(Some(c_major()), Instant::now());

    assert!(state.rotate_chord_voicing(2, PitchDirection::Up));
    assert_eq!(state.instances()[2].voicing_rotation, 1);
    assert_eq!(state.resolved_note(LaneAddress::new(2, 0)), Some(64));
    assert!(state.rotate_chord_voicing(2, PitchDirection::Down));
    assert_eq!(state.instances()[2].voicing_rotation, 0);
    assert!(state.rotate_chord_voicing(2, PitchDirection::Down));
    assert_eq!(state.instances()[2].voicing_rotation, -1);
    assert_eq!(state.resolved_note(LaneAddress::new(2, 0)), Some(55));
    assert!(state.rotate_chord_voicing(2, PitchDirection::Down));
    assert_eq!(state.instances()[2].voicing_rotation, -2);
    assert_eq!(state.resolved_note(LaneAddress::new(2, 0)), Some(52));
    assert!(state.rotate_chord_voicing(2, PitchDirection::Down));
    assert_eq!(state.instances()[2].voicing_rotation, -3);
    assert_eq!(state.resolved_note(LaneAddress::new(2, 0)), Some(48));
    for _ in 0..12 {
        assert!(state.rotate_chord_voicing(2, PitchDirection::Down));
    }
    assert_eq!(state.instances()[2].voicing_rotation, -15);
    assert_eq!(state.resolved_note(LaneAddress::new(2, 0)), Some(0));
    assert!(!state.rotate_chord_voicing(2, PitchDirection::Down));
    assert_eq!(state.instances()[2].voicing_rotation, -15);
    assert!(!state.rotate_chord_voicing(0, PitchDirection::Up));
}

#[test]
fn clear_notes_keeps_the_chord_summary_and_clears_every_other_lane() {
    let mut state = GridState::with_instance_count(3);
    state.lane_mut(primary(0)).unwrap().pattern.draw_span(0, 2);
    state
        .lane_mut(LaneAddress::new(2, 3))
        .unwrap()
        .pattern
        .draw_span(0, 2);
    state.set_chord(Some(c_major()), Instant::now());
    assert!(state.clear_notes());
    assert!(state.lane(primary(0)).unwrap().pattern.is_attack(0));
    assert_eq!(
        state.lane(LaneAddress::new(2, 3)).unwrap().pattern,
        NotePattern::default()
    );
}
