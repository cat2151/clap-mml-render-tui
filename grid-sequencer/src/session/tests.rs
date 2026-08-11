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
    // 行2は bass 行なので保存値ではなくコードの bass 音（auto voicing 無しなら無音）。
    assert_eq!(state.resolved_note(crate::LaneAddress::new(1, 0)), None);
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
fn resizing_keeps_instances_and_the_chord_voice_instances_four_lanes() {
    let mut third = instance(2, "Bass", 36);
    third.voicing_rotation = -5;
    let session = GridSequencerSession::new(
        vec![instance(0, "Piano", 60), instance(1, "Sub", 36), third],
        PatternEvolution::Hold,
    );
    let mut screen = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 4,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });
    screen.resize_for_restart(4, &[]);
    assert_eq!(screen.track_count(), 4);
    assert_eq!(screen.state.instances()[2].lanes.len(), 4);
    assert_eq!(screen.state.instances()[2].voicing_rotation, -5);
    assert_eq!(
        screen.state.instances()[2].lane_mode,
        GridLaneMode::ChordVoices4
    );
    // 足りないぶんは既定の行として足す（譜面は抽選で埋まる）。
    let added = &screen.state.instances()[3];
    assert_eq!(added.lane_mode, GridInstance::new(3).lane_mode);
    assert_eq!(added.lanes.len(), 1);
    assert_eq!(added.voicing_rotation, 0);
}

/// track 数を増やしたら、増やした行は抽選して埋める。
///
/// 空のまま足すと HOLD では譜面を引き直さないので、増やした行が無音のままになる。
#[test]
fn growing_the_track_count_fills_the_added_rows_with_notes() {
    let session = GridSequencerSession::new(
        vec![instance(0, "Piano", 60), instance(1, "Sub", 36)],
        PatternEvolution::Hold,
    );
    let mut screen = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 2,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });
    let patches = vec![("Keys/New.fxp".to_string(), "keys/new.fxp".to_string())];

    screen.resize_for_restart(4, &patches);

    for instance_index in 2..4 {
        let item = &screen.state.instances()[instance_index];
        assert_eq!(
            item.patch.as_deref(),
            Some("Keys/New.fxp"),
            "増やした行 {instance_index} に音色が付いていない"
        );
        assert!(
            item.lanes
                .iter()
                .any(|lane| (0..GRID_STEPS).any(|step| lane.pattern.is_attack(step))),
            "増やした行 {instance_index} が無音のまま"
        );
    }
    // 既存の行は触らない（HOLD で手編集した譜面を守る）。
    assert_eq!(screen.state.instances()[0].patch.as_deref(), Some("Piano"));
    assert_eq!(screen.state.instances()[0].lanes[0].base_note, 60);
}

/// bass 行が 4 voice の既定行だった頃のセッションを引き継ぐ。
#[test]
fn a_restored_bass_row_is_migrated_back_to_its_two_lanes() {
    let mut old_bass = instance(1, "Bass", 36);
    old_bass.lane_mode = GridLaneMode::ChordVoices4;
    old_bass.voicing_rotation = -3;
    old_bass.normalize();
    assert_eq!(old_bass.lanes.len(), 4);

    let session = GridSequencerSession::new(
        vec![instance(0, "Piano", 60), old_bass],
        PatternEvolution::Hold,
    );
    let screen = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 2,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });

    let bass = &screen.state.instances()[crate::BASS_ROW];
    assert_eq!(bass.lane_mode, GridLaneMode::BassOctave2);
    assert_eq!(bass.lanes.len(), 2);
    assert_eq!(bass.voicing_rotation, 0);
}

/// bass 行が 1 lane だった頃のセッションでは、octave 上の lane を空で足す。
#[test]
fn a_restored_single_lane_bass_row_gains_an_empty_octave_lane() {
    let mut old_bass = instance(1, "Bass", 36);
    old_bass.lane_mode = GridLaneMode::Single;
    old_bass.normalize();
    old_bass.lanes[0].pattern.draw_span(2, 3);

    let session = GridSequencerSession::new(
        vec![instance(0, "Piano", 60), old_bass],
        PatternEvolution::Hold,
    );
    let screen = GridSequencerScreen::new_with(crate::GridSequencerParts {
        track_count: 2,
        restored_session: Some(session),
        ..crate::GridSequencerParts::default()
    });

    let bass = &screen.state.instances()[crate::BASS_ROW];
    assert_eq!(bass.lanes.len(), 2);
    // 保存されていた root の pattern はそのまま、octave 上は空で始まる。
    assert!(bass.lanes[0].pattern.is_attack(2));
    assert!((0..GRID_STEPS).all(|step| !bass.lanes[1].pattern.is_attack(step)));
}
