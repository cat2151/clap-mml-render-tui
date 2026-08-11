use super::*;
use crate::state::{LaneAddress, CHORD_ROW};

/// 4 voice の既定行が instance 1 だった頃のセッション。
fn legacy_instances() -> Vec<GridInstance> {
    let chord = GridInstance::new(CHORD_ROW);
    let mut legacy_voices = GridInstance::new(0);
    legacy_voices.lane_mode = GridLaneMode::ChordVoices4;
    legacy_voices.voicing_rotation = -3;
    legacy_voices.normalize();
    let single = GridInstance::new(0);
    let mut fourth = GridInstance::new(0);
    fourth.lanes[0].pattern.draw_span(1, 1);
    vec![chord, legacy_voices, single, fourth]
}

/// bass 行の 4 voice を畳むだけでなく、4 voice の既定行を行3へ作り直す。
/// ここを直さないと 4 voice の行が1つも無くなり、アルペジオが書けなくなる。
#[test]
fn a_legacy_session_moves_its_chord_voice_row_to_the_default_index() {
    let mut state = GridState::with_instance_count(4);
    assert!(state.restore_instances(legacy_instances()));

    let bass = &state.instances()[crate::BASS_ROW];
    assert_eq!(bass.lane_mode, GridLaneMode::BassOctave2);
    assert_eq!(bass.lanes.len(), 2);
    assert_eq!(bass.voicing_rotation, 0);

    let voices = &state.instances()[2];
    assert_eq!(voices.lane_mode, GridLaneMode::ChordVoices4);
    assert_eq!(voices.lanes.len(), 4);
}

/// 4 voice の行が復活するので、アルペジオの声部数も戻る。
#[test]
fn the_restored_chord_voice_row_can_be_arpeggiated_again() {
    let mut state = GridState::with_instance_count(4);
    assert!(state.restore_instances(legacy_instances()));
    state.set_chord(
        crate::ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]),
        std::time::Instant::now(),
    );

    assert_eq!(state.arp_voice_count(2), 4);
}

/// lane_mode 以外の保存値（pattern）は触らない。
#[test]
fn restoring_keeps_the_saved_patterns_of_untouched_rows() {
    let mut state = GridState::with_instance_count(4);
    assert!(state.restore_instances(legacy_instances()));

    assert!(state
        .lane(LaneAddress::new(3, 0))
        .expect("lane exists")
        .pattern
        .is_attack(1));
}
