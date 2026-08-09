//! bass 行（[`BASS_ROW`]）の音高解決。行の pattern はそのまま、音高だけがコードに従う。

use super::*;

use cmrt_chord::ChordVoicing;

fn voicing(bass: u8, notes: &[u8]) -> ChordVoicing {
    ChordVoicing {
        bass: Some(bass),
        notes: notes.to_vec(),
    }
}

/// bass 付きの `C → F` 進行。auto voicing を通した実経路と同じ形。
fn voiced_c_then_f() -> ChordPlayback {
    ChordPlayback::from_voicings(
        "C",
        "I-IV".to_string(),
        vec![voicing(48, &[60, 64, 67]), voicing(53, &[60, 65, 69])],
    )
    .unwrap()
}

#[test]
fn the_bass_row_plays_the_bass_note_of_the_current_chord() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(4);
    state.set_chord(Some(voiced_c_then_f()), now);

    assert_eq!(state.resolved_note(LaneAddress::new(BASS_ROW, 0)), Some(48));

    state.advance_chord();

    assert_eq!(state.resolved_note(LaneAddress::new(BASS_ROW, 0)), Some(53));
}

/// bass は和音側に含めない。和音の行は構成音だけを鳴らす。
#[test]
fn the_chord_row_does_not_sound_the_bass_note() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(4);
    state.set_chord(Some(voiced_c_then_f()), now);
    state.start(now);

    let notes = step_at(&mut state, now)
        .into_iter()
        .filter(|message| message.instance_id == CHORD_ROW as u8 && message.message[0] == 0x90)
        .map(|message| message.message[1])
        .collect::<Vec<_>>();
    assert_eq!(notes, vec![60, 64, 67]);
}

/// bass 行は自分の pattern をそのまま使う。和音の行のように小節頭固定ではない。
#[test]
fn the_bass_row_keeps_its_own_rhythm() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(4);
    state.instances[BASS_ROW].pattern.draw_span(2, 2);
    state.set_chord(Some(voiced_c_then_f()), now);
    state.start(now);

    // step 0 と 1 では鳴らず、pattern を書いた step 2 で鳴る。
    for step in 0..2 {
        assert!(
            step_at(&mut state, at_step(now, step))
                .iter()
                .all(|message| message.instance_id != BASS_ROW as u8 || message.message[0] == 0xB0),
            "step {step} では bass 行は鳴らさない"
        );
    }
    let third = step_at(&mut state, at_step(now, 2));
    assert_eq!(
        third
            .into_iter()
            .filter(|message| message.instance_id == BASS_ROW as u8 && message.message[0] == 0x90)
            .map(|message| message.message[1])
            .collect::<Vec<_>>(),
        vec![48]
    );
}

/// bass 行に lane が複数あっても（session 復元で `ChordVoices4` のまま来た場合）、
/// 鳴らすのは lane 0 だけ。
#[test]
fn only_the_first_lane_of_the_bass_row_sounds() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(4);
    state.instances[BASS_ROW].lane_mode = crate::GridLaneMode::ChordVoices4;
    state.instances[BASS_ROW].normalize();
    state.set_chord(Some(voiced_c_then_f()), now);

    assert_eq!(state.resolved_note(LaneAddress::new(BASS_ROW, 0)), Some(48));
    for lane in 1..4 {
        assert_eq!(state.resolved_note(LaneAddress::new(BASS_ROW, lane)), None);
    }
}

/// auto voicing を通していない進行には bass が無い。行は無音のままにする。
#[test]
fn a_progression_without_a_bass_leaves_the_row_silent() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(4);
    state.set_chord(Some(c_major_then_f_major()), now);

    assert_eq!(state.resolved_note(LaneAddress::new(BASS_ROW, 0)), None);
}
