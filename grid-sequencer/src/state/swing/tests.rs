use std::time::{Duration, Instant};

use super::*;
use crate::state::{ChordPlayback, BPM, CHORD_ROW};

/// 16分1つぶんの秒数。`swing_offset_seconds` はこの何倍かで表せる。
fn step_seconds_at(bpm: f64) -> f64 {
    60.0 / (bpm * f64::from(u32::try_from(STEPS_PER_BEAT).unwrap()))
}

fn ratio_of(swing: u8, step: usize, bpm: f64) -> f64 {
    swing_offset_seconds(swing, step, bpm) / step_seconds_at(bpm)
}

#[test]
fn the_downbeats_never_move() {
    for step in (0..GRID_STEPS).step_by(2) {
        assert_eq!(
            swing_offset_seconds(SWING_MAX, step, BPM),
            0.0,
            "step {step}"
        );
    }
}

#[test]
fn fifty_percent_is_an_even_grid() {
    for step in 0..GRID_STEPS {
        assert_eq!(
            swing_offset_seconds(SWING_MIN, step, BPM),
            0.0,
            "step {step}"
        );
    }
}

/// 66% は「裏の16分が8分音符の 2/3 の位置へ来る」。ずれ幅は 16/50 step。
#[test]
fn the_offbeats_move_back_by_the_swing_ratio() {
    for step in (1..GRID_STEPS).step_by(2) {
        assert!(
            (ratio_of(SWING_MAX, step, BPM) - 0.32).abs() < 1e-12,
            "step {step}"
        );
    }
    assert!((ratio_of(58, 1, BPM) - 0.16).abs() < 1e-12);
}

/// 比率で持つので、テンポを乗り換えても跳ね具合は変わらない（秒数だけが縮む）。
#[test]
fn the_swing_ratio_survives_a_tempo_change() {
    let slow = swing_offset_seconds(SWING_MAX, 1, 90.0);
    let fast = swing_offset_seconds(SWING_MAX, 1, 180.0);
    assert!(slow > fast);
    assert!((ratio_of(SWING_MAX, 1, 90.0) - ratio_of(SWING_MAX, 1, 180.0)).abs() < 1e-12);
}

#[test]
fn out_of_range_values_are_clamped_instead_of_wrapping() {
    assert_eq!(clamp_swing(0), SWING_MIN);
    assert_eq!(clamp_swing(255), SWING_MAX);
    assert_eq!(swing_offset_seconds(0, 1, BPM), 0.0);
    assert_eq!(
        swing_offset_seconds(255, 1, BPM),
        swing_offset_seconds(SWING_MAX, 1, BPM)
    );
}

#[test]
fn a_stopped_clock_does_not_divide_by_zero() {
    assert_eq!(swing_offset_seconds(SWING_MAX, 1, 0.0), 0.0);
}

#[test]
fn a_row_with_nothing_on_the_offbeats_does_not_swing() {
    let mut state = GridState::silent();
    state.instances[0].swing = SWING_MAX;
    // 表拍だけの八分音符。跳ねる余地が無い。
    for step in (0..GRID_STEPS).step_by(2) {
        state.instances[0].lanes[0].pattern.draw_span(step, step);
    }

    assert_eq!(state.effective_swing(0), None);
}

#[test]
fn a_row_with_an_offbeat_attack_swings() {
    let mut state = GridState::silent();
    state.instances[0].swing = 60;
    state.instances[0].lanes[0].pattern.draw_span(3, 3);

    assert_eq!(state.effective_swing(0), Some(60));
}

/// 跳ねる行でも 50 なら 50 と出す。「対象外」と「対象だが等分」は別物。
#[test]
fn an_offbeat_row_left_at_fifty_is_still_reported() {
    let mut state = GridState::silent();
    state.instances[0].lanes[0].pattern.draw_span(1, 1);

    assert_eq!(state.effective_swing(0), Some(SWING_MIN));
}

#[test]
fn a_silent_row_does_not_swing() {
    let mut state = GridState::silent();
    state.instances[0].swing = SWING_MAX;

    assert_eq!(state.effective_swing(0), None);
}

/// chord 行は step 0 で1発鳴らして1meas伸ばすだけ。裏拍判定だけで自動的に外れる。
#[test]
fn the_chord_row_never_swings() {
    let now = Instant::now();
    let mut state = GridState::silent();
    state.instances[CHORD_ROW].swing = SWING_MAX;
    // chord mode の外なら普通の行なので、裏拍を描けば跳ねる。
    state.instances[CHORD_ROW].lanes[0].pattern.draw_span(1, 1);
    assert_eq!(state.effective_swing(CHORD_ROW), Some(SWING_MAX));

    state.set_chord(
        Some(ChordPlayback::new("C", "I".to_string(), vec![vec![60, 64, 67]]).unwrap()),
        now,
    );

    assert_eq!(state.effective_swing(CHORD_ROW), None);
}

/// 跳ねた行だけ後ろへずれ、跳ねない行は元の位置のまま。
#[test]
fn only_the_swung_instance_moves_within_a_step() {
    let now = Instant::now();
    let mut state = GridState::silent();
    state.instances[0].swing = SWING_MAX;
    state.instances[0].lanes[0].pattern.draw_span(1, 1);
    state.instances[1].lanes[0].pattern.draw_span(1, 1);
    state.start(now);
    // step 0 を組み立てて捨て、次の poll で step 1（裏拍）を取る。
    state.poll_steps(now, Duration::ZERO);

    let scheduled = state.poll_steps(now + crate::step_offset(1), Duration::ZERO);
    let note_on = |instance: usize| {
        scheduled
            .iter()
            .find(|item| item.message[0] == 0x90 && usize::from(item.instance_id) == instance)
            .expect("the instance sounds on this step")
    };

    let swung = note_on(0);
    let straight = note_on(1);
    let expected = swing_offset_seconds(SWING_MAX, 1, BPM);
    assert!((swung.timeline_seconds - straight.timeline_seconds - expected).abs() < 1e-9);
    assert_eq!(
        swung.ahead - straight.ahead,
        Duration::from_secs_f64(expected)
    );
}

/// 裏拍で鳴らした音は、次の step の note off に追い越されない（＝即消えない）。
#[test]
fn a_swung_note_is_not_overtaken_by_its_own_release() {
    let now = Instant::now();
    let mut state = GridState::silent();
    state.instances[0].swing = SWING_MAX;
    state.instances[0].lanes[0].pattern.draw_span(1, 1);
    state.start(now);

    let mut on = None;
    let mut off = None;
    for step in 0..4 {
        for item in state.poll_steps(now + crate::step_offset(step), Duration::ZERO) {
            match item.message[0] {
                0x90 => on = Some(item.timeline_seconds),
                0x80 => off = Some(item.timeline_seconds),
                _ => {}
            }
        }
    }

    let on = on.expect("note on");
    let off = off.expect("note off");
    assert!(off > on, "on={on} off={off}");
}
