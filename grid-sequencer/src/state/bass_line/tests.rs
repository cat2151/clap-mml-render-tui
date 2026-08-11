use std::time::Instant;

use cmrt_arpeggiator::{generate_bass_line, BassNote, BassPattern};
use cmrt_chord::ChordVoicing;

use super::*;
use crate::{ChordPlayback, GridLaneMode, LaneAddress, NoteStep, GRID_STEPS};

/// bass 付きの C。bass 行は auto voicing を通した進行でしか鳴らない。
fn c_major() -> ChordPlayback {
    ChordPlayback::from_voicings(
        "C",
        "I".to_string(),
        vec![ChordVoicing {
            bass: Some(48),
            notes: vec![60, 64, 67],
        }],
    )
    .unwrap()
}

/// 和音・bass・4 voice・Single lane の instance を1つずつ持つ state。
fn chorded_state() -> GridState {
    let mut state = GridState::with_instance_count(4);
    state.set_chord(Some(c_major()), Instant::now());
    state
}

fn note(step: usize, voice: usize, duration_steps: usize) -> BassNote {
    BassNote {
        step,
        voice,
        duration_steps,
    }
}

fn lane_pattern(state: &GridState, lane: usize) -> String {
    state
        .lane(LaneAddress::new(BASS_ROW, lane))
        .expect("lane exists")
        .pattern
        .steps()
        .iter()
        .map(|step| match step {
            NoteStep::Rest => '.',
            NoteStep::Attack => '#',
            NoteStep::Tie => '-',
        })
        .collect()
}

#[test]
fn the_bass_row_has_a_root_lane_and_an_octave_lane() {
    let state = chorded_state();
    assert_eq!(
        state.instances()[BASS_ROW].lane_mode,
        GridLaneMode::BassOctave2
    );
    assert_eq!(state.instances()[BASS_ROW].lanes.len(), 2);
}

#[test]
fn voices_are_written_to_the_matching_lanes() {
    let mut state = chorded_state();
    assert!(state.apply_bass_line(&[note(0, 0, 2), note(2, 1, 2), note(4, 0, 1)]));
    assert_eq!(lane_pattern(&state, 0), "#-..#...........");
    assert_eq!(lane_pattern(&state, 1), "..#-............");
}

#[test]
fn the_previous_pattern_of_every_lane_is_discarded() {
    let mut state = chorded_state();
    for lane in 0..2 {
        state
            .lane_mut(LaneAddress::new(BASS_ROW, lane))
            .unwrap()
            .pattern
            .draw_span(0, GRID_STEPS - 1);
    }
    assert!(state.apply_bass_line(&[note(0, 0, 1)]));
    assert_eq!(lane_pattern(&state, 0), "#...............");
    assert_eq!(lane_pattern(&state, 1), "................");
}

#[test]
fn a_chordless_grid_is_rejected() {
    let mut chordless = GridState::with_instance_count(4);
    assert!(!chordless.bass_line_is_available());
    assert!(!chordless.apply_bass_line(&[note(0, 0, 1)]));
}

#[test]
fn a_grid_without_a_bass_row_is_rejected() {
    let mut state = GridState::with_instance_count(1);
    state.set_chord(Some(c_major()), Instant::now());
    assert!(!state.bass_line_is_available());
    assert!(!state.apply_bass_line(&[note(0, 0, 1)]));
}

#[test]
fn an_unchanged_grid_reports_no_edit() {
    let mut state = chorded_state();
    assert!(!state.apply_bass_line(&[]));
    assert!(state.apply_bass_line(&[note(0, 0, 1)]));
    assert!(!state.apply_bass_line(&[note(0, 0, 1)]));
}

#[test]
fn voices_without_a_lane_are_dropped() {
    let mut state = chorded_state();
    assert!(state.apply_bass_line(&[note(0, 0, 1), note(1, 9, 1)]));
    assert_eq!(lane_pattern(&state, 0), "#...............");
    assert_eq!(lane_pattern(&state, 1), "................");
}

#[test]
fn a_legacy_single_lane_bass_row_drops_the_octave_voice_instead_of_panicking() {
    let mut state = chorded_state();
    // 保存値が 1 lane だった頃の形。`normalize()` を通らない経路でも落ちないこと。
    state.instances_mut()[BASS_ROW].lanes.truncate(1);
    assert!(state.apply_bass_line(&[note(0, 0, 2), note(2, 1, 2)]));
    assert_eq!(lane_pattern(&state, 0), "#-..............");
}

#[test]
fn a_generated_octave_line_alternates_between_the_two_lanes() {
    let mut state = chorded_state();
    let notes = generate_bass_line(BassPattern::EighthOctave, GRID_STEPS);
    assert!(state.apply_bass_line(&notes));
    assert_eq!(lane_pattern(&state, 0), "#-..#-..#-..#-..");
    assert_eq!(lane_pattern(&state, 1), "..#-..#-..#-..#-");
}

#[test]
fn a_generated_whole_note_only_uses_the_root_lane() {
    let mut state = chorded_state();
    let notes = generate_bass_line(BassPattern::Whole, GRID_STEPS);
    assert!(state.apply_bass_line(&notes));
    assert_eq!(lane_pattern(&state, 0), "#---------------");
    assert_eq!(lane_pattern(&state, 1), "................");
}

#[test]
fn applying_a_bass_line_refreshes_the_velocity_display_mask() {
    let mut state = chorded_state();
    assert!(state.apply_bass_line(&[note(4, 1, 1)]));
    let lane_index = state
        .stored_lane_index(LaneAddress::new(BASS_ROW, 1))
        .expect("stored lane");
    assert!(state.velocity_display()[lane_index][4].is_some());
}
