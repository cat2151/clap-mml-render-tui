use super::*;
use crate::{ChordPlayback, GridState, VisibleRowKind};
use std::time::Instant;

#[test]
fn second_instance_defaults_to_four_chord_voice_lanes() {
    let instances = (0..3).map(GridInstance::new).collect::<Vec<_>>();
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.lanes.len())
            .collect::<Vec<_>>(),
        vec![1, 4, 1]
    );
    assert_eq!(instances[1].lane_mode, GridLaneMode::ChordVoices4);
}

#[test]
fn normalization_fills_and_truncates_lanes_to_the_mode_capacity() {
    let mut four = GridInstance {
        patch: None,
        lane_mode: GridLaneMode::ChordVoices4,
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
fn chord_mode_expands_only_the_second_instance_and_preserves_hidden_lanes() {
    let mut state = GridState::with_instance_count(2);
    state.instances_mut()[1].lanes[3].pattern.draw_span(5, 6);
    assert_eq!(state.stored_lane_count(), 5);
    assert_eq!(state.visible_lane_count(), 2);

    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    state.set_chord(Some(chord), Instant::now());
    let rows = state.visible_note_rows();
    assert_eq!(rows.len(), 5);
    assert_eq!(state.cc1_display().len(), 2);
    assert_eq!(state.visible_velocity_display().len(), 5);
    assert_eq!(rows[0].kind, VisibleRowKind::ChordSummary);
    assert_eq!(
        rows[1..].iter().map(|row| row.address).collect::<Vec<_>>(),
        (0..4)
            .rev()
            .map(|lane| LaneAddress::new(1, lane))
            .collect::<Vec<_>>()
    );

    state.set_chord(None, Instant::now());
    assert_eq!(state.visible_lane_count(), 2);
    assert_eq!(state.visible_velocity_display().len(), 2);
    assert_eq!(
        state
            .lane(LaneAddress::new(1, 3))
            .unwrap()
            .pattern
            .attack_len(5),
        Some(2)
    );
}
