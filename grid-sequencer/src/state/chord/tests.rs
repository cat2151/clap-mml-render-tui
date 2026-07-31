use std::time::Duration;

use super::*;
use crate::state::{step_offset, GridScheduledMessage, StepDuration, GRID_STEPS};

/// 先読みなしで1ステップだけ取り出す。
fn step_at(state: &mut GridState, now: Instant) -> Vec<GridScheduledMessage> {
    state.poll_steps(now, Duration::ZERO)
}

fn messages(scheduled: &[GridScheduledMessage]) -> Vec<[u8; 3]> {
    scheduled.iter().map(|item| item.message).collect()
}

fn at_step(now: Instant, step: u64) -> Instant {
    now + step_offset(step)
}

fn c_major_then_f_major() -> ChordPlayback {
    ChordPlayback::new(
        "C",
        "I-IV".to_string(),
        vec![vec![60, 64, 67], vec![65, 69, 72]],
    )
    .unwrap()
}

#[test]
fn an_empty_progression_cannot_be_played() {
    assert!(ChordPlayback::new("C", "I".to_string(), Vec::new()).is_none());
}

#[test]
fn the_chord_row_sounds_only_on_the_first_step() {
    let now = Instant::now();
    let mut state = GridState::default();
    // 和音の行はセルを無視する。全ステップを on にしても鳴るのは先頭だけ。
    state.rows[CHORD_ROW].cells = [true; GRID_STEPS];
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);

    assert_eq!(
        messages(&step_at(&mut state, now)),
        vec![[0x90, 60, 100], [0x90, 64, 100], [0x90, 67, 100]]
    );
    for step in 1..GRID_STEPS as u64 {
        assert!(
            step_at(&mut state, at_step(now, step)).is_empty(),
            "step {step} では和音の行は何も送らない"
        );
    }
}

#[test]
fn the_chord_is_held_for_a_whole_note_and_replaced_by_the_next_chord() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);
    for step in 1..GRID_STEPS as u64 {
        step_at(&mut state, at_step(now, step));
    }

    // 16ステップ目（＝ grid 1周）で全音符が切れ、次のコードへ張り替わる。
    let wrapped = step_at(&mut state, at_step(now, GRID_STEPS as u64));

    assert_eq!(
        messages(&wrapped),
        vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x80, 67, 0],
            [0x90, 65, 100],
            [0x90, 69, 100],
            [0x90, 72, 100],
        ]
    );
    assert!(wrapped
        .iter()
        .all(|item| item.instance_id == CHORD_ROW as u8));
    assert_eq!(state.chord().unwrap().index(), 1);
}

#[test]
fn completing_the_progression_raises_the_reroll_signal_once() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);

    // 1周目の終わり（2コード目へ）ではまだ合図は立たない。
    for step in 1..=GRID_STEPS as u64 {
        step_at(&mut state, at_step(now, step));
    }
    assert!(!state.take_chord_cycle_completed());

    // 2コード目を鳴らし終えて先頭へ戻ったところで立つ。
    for step in GRID_STEPS as u64 + 1..=(GRID_STEPS as u64 * 2) {
        step_at(&mut state, at_step(now, step));
    }
    assert!(state.take_chord_cycle_completed());
    assert!(!state.take_chord_cycle_completed(), "合図は取ったら降ろす");
    assert_eq!(state.chord().unwrap().index(), 0);
}

/// 1周し終えたら、画面側が引き直すまで次の和音を鳴らし始めない
/// （引き直しは音色ロードを伴い、どのみち演奏が止まるため）。
#[test]
fn no_chord_is_attacked_while_a_reroll_is_pending() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);

    // 2コードぶん（= 進行1周）進める。
    let mut wrapped = Vec::new();
    for step in 1..=(GRID_STEPS as u64 * 2) {
        wrapped = messages(&step_at(&mut state, at_step(now, step)));
    }

    assert!(state.chord_reroll_pending());
    assert_eq!(
        wrapped,
        vec![[0x80, 65, 0], [0x80, 69, 0], [0x80, 72, 0]],
        "最後のコードの note off だけが出て、次の和音は鳴らさない"
    );

    // 引き直すと合図が降り、また鳴り始める。
    assert!(state.take_chord_cycle_completed());
    state.set_chord(Some(c_major_then_f_major()), now);
    assert!(!state.chord_reroll_pending());
}

#[test]
fn other_rows_snap_to_the_chord_while_keeping_their_octave() {
    let now = Instant::now();
    let mut state = GridState::default();
    // C2 付近と C6 付近。C major に寄せても元の音域から離れないこと。
    state.rows[1].base_note = 38;
    state.rows[2].base_note = 81;
    state.rows[3].base_note = 67;

    state.set_chord(Some(c_major_then_f_major()), now);

    assert_eq!(state.rows[1].note, 36, "38 は下の C(36) が最も近い");
    assert_eq!(state.rows[2].note, 79, "81 は下の G(79) が最も近い");
    assert_eq!(state.rows[3].note, 67, "既に構成音ならそのまま");
}

#[test]
fn other_rows_follow_the_chord_change() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.rows[1].base_note = 64;
    state.set_chord(Some(c_major_then_f_major()), now);
    assert_eq!(state.rows[1].note, 64, "C major では E(64) はそのまま");

    state.advance_chord();

    assert_eq!(state.rows[1].note, 65, "F major では E(64) は F(65) へ");

    state.advance_chord();

    assert_eq!(state.rows[1].note, 64, "1周して C major へ戻る");
}

#[test]
fn turning_the_chord_mode_off_restores_the_base_notes() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.rows[1].base_note = 38;
    state.set_chord(Some(c_major_then_f_major()), now);
    assert_eq!(state.rows[1].note, 36);

    state.set_chord(None, now);

    assert_eq!(state.rows[1].note, 38);
    assert!(state.chord().is_none());
}

#[test]
fn switching_the_chord_mode_silences_the_sounding_notes() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);

    let note_offs = state.set_chord(None, now);

    assert_eq!(
        messages(&note_offs),
        vec![[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0]]
    );
}

#[test]
fn other_rows_keep_playing_their_own_rhythm_under_the_chord() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.rows[1].base_note = 62;
    state.rows[1].duration = StepDuration::Sixteenth;
    state.rows[1].cells[1] = true;
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);

    let second = step_at(&mut state, at_step(now, 1));

    assert_eq!(messages(&second), vec![[0x90, 60, 100]]);
    assert_eq!(second[0].instance_id, 1);
}

#[test]
fn snapping_picks_the_lower_note_on_a_tie() {
    let classes = {
        let mut classes = [false; 12];
        classes[0] = true;
        classes[6] = true;
        classes
    };
    // 63 は C(60) からも F#(66) からも3半音。低いほうを選ぶ。
    assert_eq!(snap_to_chord(63, &classes), 60);
}

#[test]
fn snapping_without_any_pitch_class_keeps_the_base_note() {
    assert_eq!(snap_to_chord(60, &[false; 12]), 60);
}
