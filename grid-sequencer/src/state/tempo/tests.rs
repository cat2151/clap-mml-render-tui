use std::time::{Duration, Instant};

use crate::state::{step_offset, ChordPlayback, GridState, GRID_STEPS};

/// drum 行を含まない最小構成。テンポの境目だけを見るので譜面は問わない。
fn state_with_chords(chord_count: usize) -> GridState {
    let mut state = GridState::with_instance_count(3);
    let chords = (0..chord_count)
        .map(|index| vec![60 + (index as u8) * 2])
        .collect::<Vec<_>>();
    let progression = ChordPlayback::new("C", "I".to_string(), chords).expect("空でない進行");
    state.set_chord(Some(progression), Instant::now());
    state
}

/// 締切ちょうどで1ステップずつ取り出す。先読みを挟まないので境目が step 番号と一致する。
fn poll_step(state: &mut GridState, now: Instant, step: u64) {
    state.poll_steps(now + step_offset(step), Duration::ZERO);
}

/// テンポの引き直しは小節（grid 1周）ではなくコード進行1周ごと。
///
/// 小節ごとに変えると進行の途中でテンポが動き、同じコード進行が別々の速さで
/// 演奏されてフレーズが繋がらない。
#[test]
fn the_armed_tempo_waits_for_the_progression_to_wrap() {
    let now = Instant::now();
    let mut state = state_with_chords(2);
    state.start(now);
    state.arm_next_cycle_bpm(90.0);

    // 2コード進行の1小節目・2小節目は進行の途中なので乗り換えない。
    for step in 0..2 * GRID_STEPS as u64 {
        poll_step(&mut state, now, step);
        assert!(
            state.take_applied_cycle_bpm().is_none(),
            "step {step} で乗り換えた（小節ごとに変わっている）"
        );
    }

    // 進行が先頭のコードへ戻る境目で初めて乗り換える。
    poll_step(&mut state, now, 2 * GRID_STEPS as u64);
    let applied = state
        .take_applied_cycle_bpm()
        .expect("進行を1周しても乗り換えていない");
    assert_eq!(applied.bpm, 90.0);
}

/// 4コード進行なら4小節に1回。進行の長さがそのまま引き直しの周期になる。
#[test]
fn a_longer_progression_stretches_the_tempo_cycle() {
    let now = Instant::now();
    let mut state = state_with_chords(4);
    state.start(now);
    state.arm_next_cycle_bpm(90.0);

    for step in 0..4 * GRID_STEPS as u64 {
        poll_step(&mut state, now, step);
        assert!(state.take_applied_cycle_bpm().is_none(), "step {step}");
    }
    poll_step(&mut state, now, 4 * GRID_STEPS as u64);
    assert!(state.take_applied_cycle_bpm().is_some());
}

/// chord mode を使っていない間は「進行1周」という単位が無いので、従来どおり
/// grid 1周を単位にする。
#[test]
fn without_a_progression_the_tempo_still_moves_every_grid_wrap() {
    let now = Instant::now();
    let mut state = GridState::with_instance_count(3);
    state.start(now);
    state.arm_next_cycle_bpm(90.0);

    for step in 0..GRID_STEPS as u64 {
        poll_step(&mut state, now, step);
        assert!(state.take_applied_cycle_bpm().is_none(), "step {step}");
    }
    poll_step(&mut state, now, GRID_STEPS as u64);
    let applied = state
        .take_applied_cycle_bpm()
        .expect("grid を1周しても乗り換えていない");
    assert_eq!(applied.bpm, 90.0);
}
