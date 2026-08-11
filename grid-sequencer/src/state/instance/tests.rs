use super::*;
use crate::{ChordPlayback, GridState, VisibleRowKind};
use std::time::Instant;

#[test]
fn the_bass_and_chord_voice_rows_default_to_their_own_lane_modes() {
    // 行1 = chord、行2 = bass は chord mode が占有するので、4声は行3が既定。
    let instances = (0..4).map(GridInstance::new).collect::<Vec<_>>();
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.lanes.len())
            .collect::<Vec<_>>(),
        vec![1, 2, 4, 1]
    );
    assert_eq!(instances[1].lane_mode, GridLaneMode::BassOctave2);
    assert_eq!(instances[2].lane_mode, GridLaneMode::ChordVoices4);
}

#[test]
fn only_the_stacked_modes_flip_the_display_order() {
    assert!(!GridLaneMode::Single.stacks_high_notes_on_top());
    assert!(GridLaneMode::BassOctave2.stacks_high_notes_on_top());
    assert!(GridLaneMode::ChordVoices4.stacks_high_notes_on_top());
    assert!(!GridLaneMode::Drum.stacks_high_notes_on_top());
}

#[test]
fn normalization_fills_and_truncates_lanes_to_the_mode_capacity() {
    let mut four = GridInstance {
        patch: None,
        lane_mode: GridLaneMode::ChordVoices4,
        drum: None,
        voicing_rotation: 0,
        lanes: vec![GridLane::default()],
    };
    four.normalize();
    assert_eq!(four.lanes.len(), 4);

    four.lane_mode = GridLaneMode::Single;
    four.normalize();
    assert_eq!(four.lanes.len(), 1);
}

#[test]
fn chord_mode_expands_only_the_chord_voice_instance_and_preserves_hidden_lanes() {
    let mut state = GridState::with_instance_count(3);
    state.instances_mut()[2].lanes[3].pattern.draw_span(5, 6);
    assert_eq!(state.stored_lane_count(), 7);
    assert_eq!(state.visible_lane_count(), 3);

    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    state.set_chord(Some(chord), Instant::now());
    let rows = state.visible_note_rows();
    assert_eq!(rows.len(), 7);
    assert_eq!(state.cc1_display().len(), 3);
    assert_eq!(state.visible_velocity_display().len(), 7);
    assert_eq!(rows[0].kind, VisibleRowKind::ChordSummary);
    // 行2（bass）は octave 上が上段、root が下段。
    assert_eq!(rows[1].address, LaneAddress::new(1, 1));
    assert_eq!(rows[2].address, LaneAddress::new(1, 0));
    assert_eq!(
        rows[3..].iter().map(|row| row.address).collect::<Vec<_>>(),
        (0..4)
            .rev()
            .map(|lane| LaneAddress::new(2, lane))
            .collect::<Vec<_>>()
    );

    state.set_chord(None, Instant::now());
    assert_eq!(state.visible_lane_count(), 3);
    assert_eq!(state.visible_velocity_display().len(), 3);
    assert_eq!(
        state
            .lane(LaneAddress::new(2, 3))
            .unwrap()
            .pattern
            .attack_len(5),
        Some(2)
    );
}
