use std::time::Instant;

use super::*;
use crate::{
    tests::{ctx_with, empty_catalog},
    ChordPlayback, GridLane, GridLaneMode, GridPatchLoad, NoVoicingLookup, NotePattern, NoteStep,
    GRID_STEPS,
};

fn instance(index: usize, patch: &str, note: u8) -> GridInstance {
    let mut instance = GridInstance::new(index);
    instance.patch = Some(patch.to_string());
    instance.lanes[0] = GridLane {
        base_note: note,
        pattern: NotePattern::from_steps([NoteStep::Attack; GRID_STEPS]),
    };
    instance
}

#[test]
fn restored_notes_are_derived_from_base_note_and_current_chord() {
    let mut state = GridState::with_instance_count(3);
    state.set_chord(
        ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        Instant::now(),
    );
    assert!(state.restore_instances(vec![
        instance(0, "Piano", 60),
        instance(1, "Bass", 62),
        instance(2, "Lead", 62),
    ]));
    assert_eq!(state.resolved_note(crate::LaneAddress::new(1, 0)), Some(60));
    assert_eq!(state.resolved_note(crate::LaneAddress::new(2, 0)), Some(60));
}

#[test]
fn ready_patch_catalog_replaces_only_disappeared_saved_patches() {
    let session = GridSequencerSession::new(
        vec![instance(0, "Still Here", 60), instance(1, "Gone", 62)],
        PatternEvolution::Hold,
    );
    let mut screen = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 2,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });
    let patches = vec![("Still Here".to_string(), "a".to_string())];
    let ctx = ctx_with(
        GridPatchLoad::Ready(&patches),
        empty_catalog(),
        &NoVoicingLookup,
    );
    screen.enter(Instant::now(), &ctx);
    assert_eq!(
        screen.state.instances()[0].patch.as_deref(),
        Some("Still Here")
    );
    assert_eq!(
        screen.state.instances()[1].patch.as_deref(),
        Some("Still Here")
    );
}

#[test]
fn resizing_keeps_instances_and_the_second_instances_four_lanes() {
    let mut second = instance(1, "Bass", 36);
    second.voicing_rotation = -5;
    let session = GridSequencerSession::new(
        vec![instance(0, "Piano", 60), second],
        PatternEvolution::Hold,
    );
    let mut screen = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 2,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });
    screen.resize_for_restart(4);
    assert_eq!(screen.track_count(), 4);
    assert_eq!(screen.state.instances()[1].lanes.len(), 4);
    assert_eq!(screen.state.instances()[1].voicing_rotation, -5);
    assert_eq!(
        screen.state.instances()[1].lane_mode,
        GridLaneMode::ChordVoices4
    );
    assert_eq!(screen.state.instances()[2], GridInstance::new(2));
    assert_eq!(screen.state.instances()[3], GridInstance::new(3));
}
