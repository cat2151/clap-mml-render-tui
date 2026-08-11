use std::time::Instant;

use cmrt_arpeggiator::{generate_arpeggio, ArpNote, ArpPattern};

use super::*;
use crate::{ChordPlayback, NoteStep, GRID_STEPS};

fn c_major() -> ChordPlayback {
    ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap()
}

/// 和音・bass・4 voice・Single lane の instance を1つずつ持つ state。
fn chorded_state() -> GridState {
    let mut state = GridState::with_instance_count(4);
    state.set_chord(Some(c_major()), Instant::now());
    state
}

fn note(step: usize, voice: usize, duration_steps: usize) -> ArpNote {
    ArpNote {
        step,
        voice,
        duration_steps,
    }
}

fn lane_steps(state: &GridState, instance: usize, lane: usize) -> Vec<NoteStep> {
    state
        .lane(LaneAddress::new(instance, lane))
        .expect("lane exists")
        .pattern
        .steps()
        .to_vec()
}

#[test]
fn voice_count_follows_the_chord_voices_of_the_instance() {
    let state = chorded_state();
    // 三和音は4 voiceへ展開される（lane 3 が root のオクターブ上）。
    assert_eq!(state.arp_voice_count(2), 4);
    // Single lane は声部が1つしかないのでアルペジオにならない。
    assert_eq!(state.arp_voice_count(3), 1);
    // 和音の行・bass の行・範囲外は対象外。
    assert_eq!(state.arp_voice_count(CHORD_ROW), 0);
    assert_eq!(state.arp_voice_count(BASS_ROW), 0);
    assert_eq!(state.arp_voice_count(9), 0);
}

#[test]
fn voice_count_is_zero_without_a_chord() {
    let state = GridState::with_instance_count(4);
    assert_eq!(state.arp_voice_count(2), 0);
    assert_eq!(state.arp_voice_count(3), 0);
}

#[test]
fn a_two_note_chord_only_offers_the_voices_it_can_render() {
    let mut state = GridState::with_instance_count(3);
    let dyad = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 67]]).unwrap();
    state.set_chord(Some(dyad), Instant::now());
    assert_eq!(state.arp_voice_count(2), 2);
}

#[test]
fn voices_are_written_to_the_matching_lanes() {
    let mut state = chorded_state();
    let notes = [note(0, 0, 2), note(2, 1, 1), note(3, 3, 4)];
    assert!(state.apply_arpeggio(2, &notes));

    let lane0 = lane_steps(&state, 2, 0);
    assert_eq!(lane0[0], NoteStep::Attack);
    assert_eq!(lane0[1], NoteStep::Tie);
    assert_eq!(lane0[2], NoteStep::Rest);

    let lane1 = lane_steps(&state, 2, 1);
    assert_eq!(lane1[2], NoteStep::Attack);
    assert_eq!(lane1[3], NoteStep::Rest);

    let lane3 = lane_steps(&state, 2, 3);
    assert_eq!(lane3[3], NoteStep::Attack);
    assert_eq!(&lane3[4..7], [NoteStep::Tie; 3]);
    assert_eq!(lane3[7], NoteStep::Rest);

    // 触れていない voice 2 の lane は空のまま。
    assert!(lane_steps(&state, 2, 2)
        .iter()
        .all(|step| *step == NoteStep::Rest));
}

#[test]
fn the_previous_pattern_of_every_lane_is_discarded() {
    let mut state = chorded_state();
    for lane in 0..4 {
        state
            .lane_mut(LaneAddress::new(2, lane))
            .unwrap()
            .pattern
            .draw_span(0, GRID_STEPS - 1);
    }
    assert!(state.apply_arpeggio(2, &[note(0, 0, 1)]));
    for lane in 1..4 {
        assert!(
            lane_steps(&state, 2, lane)
                .iter()
                .all(|step| *step == NoteStep::Rest),
            "lane {lane} kept its old pattern"
        );
    }
    assert_eq!(lane_steps(&state, 2, 0)[0], NoteStep::Attack);
    assert_eq!(lane_steps(&state, 2, 0)[1], NoteStep::Rest);
}

#[test]
fn the_chord_summary_and_a_chordless_grid_are_rejected() {
    let mut state = chorded_state();
    assert!(!state.apply_arpeggio(CHORD_ROW, &[note(0, 0, 1)]));
    assert!(!state.apply_arpeggio(9, &[note(0, 0, 1)]));

    assert!(!state.apply_arpeggio(BASS_ROW, &[note(0, 0, 1)]));

    let mut chordless = GridState::with_instance_count(3);
    assert!(!chordless.apply_arpeggio(2, &[note(0, 0, 1)]));
}

#[test]
fn an_unchanged_grid_reports_no_edit() {
    let mut state = chorded_state();
    assert!(!state.apply_arpeggio(2, &[]));
    assert!(state.apply_arpeggio(2, &[note(0, 0, 1)]));
    assert!(!state.apply_arpeggio(2, &[note(0, 0, 1)]));
}

#[test]
fn voices_without_a_lane_are_dropped() {
    let mut state = chorded_state();
    assert!(state.apply_arpeggio(2, &[note(0, 0, 1), note(1, 9, 1)]));
    assert_eq!(lane_steps(&state, 2, 0)[0], NoteStep::Attack);
}

#[test]
fn a_generated_arpeggio_lands_on_every_step_of_the_instance() {
    let mut state = chorded_state();
    let voice_count = state.arp_voice_count(2);
    let notes = generate_arpeggio(ArpPattern::Up, voice_count, GRID_STEPS, &mut rand::rng());
    assert!(state.apply_arpeggio(2, &notes));

    // up arpeggioは lane 0..3 を順に叩くので、各 step にちょうど1つ Attack が立つ。
    for step in 0..GRID_STEPS {
        let attacks = (0..voice_count)
            .filter(|lane| lane_steps(&state, 2, *lane)[step] == NoteStep::Attack)
            .count();
        assert_eq!(attacks, 1, "step {step} did not get exactly one attack");
    }
}

#[test]
fn applying_an_arpeggio_refreshes_the_velocity_display_mask() {
    let mut state = chorded_state();
    assert!(state.apply_arpeggio(2, &[note(4, 2, 1)]));
    let lane_index = state
        .stored_lane_index(LaneAddress::new(2, 2))
        .expect("stored lane");
    assert!(state.velocity_display()[lane_index][4].is_some());
}
