use super::*;
use crate::{ChordPlayback, GridState, VisibleRowKind};
use std::time::Instant;

#[test]
fn third_instance_defaults_to_four_chord_voice_lanes() {
    // 行1 = chord、行2 = bass は chord mode が占有するので、4声は行3が既定。
    let instances = (0..4).map(GridInstance::new).collect::<Vec<_>>();
    assert_eq!(
        instances
            .iter()
            .map(|instance| instance.lanes.len())
            .collect::<Vec<_>>(),
        vec![1, 1, 4, 1]
    );
    assert_eq!(instances[2].lane_mode, GridLaneMode::ChordVoices4);
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
fn chord_mode_expands_only_the_chord_voice_instance_and_preserves_hidden_lanes() {
    let mut state = GridState::with_instance_count(3);
    state.instances_mut()[2].lanes[3].pattern.draw_span(5, 6);
    assert_eq!(state.stored_lane_count(), 6);
    assert_eq!(state.visible_lane_count(), 3);

    let chord = ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap();
    state.set_chord(Some(chord), Instant::now());
    let rows = state.visible_note_rows();
    assert_eq!(rows.len(), 6);
    assert_eq!(state.cc1_display().len(), 3);
    assert_eq!(state.visible_velocity_display().len(), 6);
    assert_eq!(rows[0].kind, VisibleRowKind::ChordSummary);
    // 行2（bass）は Single lane なので1行のまま。
    assert_eq!(rows[1].address, LaneAddress::new(1, 0));
    assert_eq!(
        rows[2..].iter().map(|row| row.address).collect::<Vec<_>>(),
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
