use std::time::Duration;

use super::velocity::normalize_velocity;
use super::*;

/// 先読みなしで1ステップだけ取り出す（従来の「締切が来たら送る」と同じ挙動）。
fn step_at(state: &mut GridState, now: Instant) -> Vec<[u8; 3]> {
    state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .filter(|scheduled| scheduled.message[0] != 0xB0)
        .map(|scheduled| normalize_velocity(scheduled.message))
        .collect()
}

/// `step_at` と同じ取り出し方で、instance と先読み時間まで見たいとき用。
fn scheduled_at(state: &mut GridState, now: Instant) -> Vec<GridScheduledMessage> {
    state
        .poll_steps(now, Duration::ZERO)
        .into_iter()
        .filter(|scheduled| scheduled.message[0] != 0xB0)
        .map(|scheduled| GridScheduledMessage {
            message: normalize_velocity(scheduled.message),
            ..scheduled
        })
        .collect()
}

/// `step` ステップ目の絶対位置。`STEP_INTERVAL * step` では丸め誤差が積もってずれる。
fn at_step(now: Instant, step: u64) -> Instant {
    now + step_offset(step)
}

#[test]
fn the_first_step_sounds_immediately_on_entry() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);

    assert_eq!(step_at(&mut state, now), vec![[0x90, 60, 100]]);
    assert_eq!(state.step_index(), 0);
}

#[test]
fn a_stopped_state_produces_nothing() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].pattern.draw_span(0, 0);

    assert!(!state.is_running());
    assert!(step_at(&mut state, now).is_empty());
}

#[test]
fn a_sixteenth_note_is_released_on_the_next_step() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].base_note = 62;
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);

    assert_eq!(step_at(&mut state, now), vec![[0x90, 62, 100]]);
    assert_eq!(step_at(&mut state, at_step(now, 1)), vec![[0x80, 62, 0]]);
    assert!(step_at(&mut state, at_step(now, 2)).is_empty());
}

#[test]
fn a_quarter_note_sustains_for_four_steps() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].base_note = 65;
    state.instances[0].pattern.draw_span(0, 3);
    state.start(now);

    assert_eq!(step_at(&mut state, now), vec![[0x90, 65, 100]]);
    for step in 1..=3 {
        assert!(step_at(&mut state, at_step(now, step)).is_empty());
    }
    assert_eq!(step_at(&mut state, at_step(now, 4)), vec![[0x80, 65, 0]]);
}

#[test]
fn rows_sharing_a_step_sound_together_in_row_order() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].base_note = 60;
    state.instances[0].pattern.draw_span(0, 0);
    state.instances[3].base_note = 64;
    state.instances[3].pattern.draw_span(0, 0);
    state.start(now);

    assert_eq!(
        step_at(&mut state, now),
        vec![[0x90, 60, 100], [0x90, 64, 100]]
    );
}

/// 同じステップの音は同じ offset で送る。これが揃わないと頭がばらける。
#[test]
fn messages_of_one_step_share_the_same_offset() {
    let now = Instant::now();
    let mut state = GridState::default();
    for row in 0..4 {
        state.instances[row].base_note = 60 + row as u8;
        state.instances[row].pattern.draw_span(1, 1);
    }
    state.start(now);
    state.poll_steps(now, Duration::ZERO);

    let scheduled = state.poll_steps(now, STEP_INTERVAL);

    // 全行へ毎step送る CC1 と、4行ぶんの note on。
    assert_eq!(scheduled.len(), GRID_ROWS + 4);
    assert!(scheduled
        .iter()
        .all(|item| item.ahead == scheduled[0].ahead));
    assert_eq!(scheduled[0].ahead, STEP_INTERVAL);
}

#[test]
fn equal_note_numbers_on_different_rows_are_independent() {
    let now = Instant::now();
    let mut state = GridState::default();
    // 行0の4分音符が鳴り続けている間に、別instanceの行1で同じnoteを鳴らす。
    state.instances[0].base_note = 67;
    state.instances[0].pattern.draw_span(0, 3);
    state.instances[1].base_note = 67;
    state.instances[1].pattern.draw_span(1, 1);
    state.start(now);

    let first = scheduled_at(&mut state, now);
    assert_eq!(
        first,
        vec![GridScheduledMessage {
            instance_id: 0,
            ahead: Duration::ZERO,
            message: [0x90, 67, 100],
        }]
    );

    let second = scheduled_at(&mut state, at_step(now, 1));
    assert_eq!(
        second,
        vec![GridScheduledMessage {
            instance_id: 1,
            ahead: Duration::ZERO,
            message: [0x90, 67, 100],
        }]
    );
    assert_eq!(
        scheduled_at(&mut state, at_step(now, 2)),
        vec![GridScheduledMessage {
            instance_id: 1,
            ahead: Duration::ZERO,
            message: [0x80, 67, 0],
        }]
    );
    assert!(scheduled_at(&mut state, at_step(now, 3)).is_empty());
    assert_eq!(
        scheduled_at(&mut state, at_step(now, 4)),
        vec![GridScheduledMessage {
            instance_id: 0,
            ahead: Duration::ZERO,
            message: [0x80, 67, 0],
        }]
    );
}

#[test]
fn the_playhead_wraps_after_sixteen_steps() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);

    assert_eq!(step_at(&mut state, now), vec![[0x90, 60, 100]]);
    for step in 1..GRID_STEPS as u64 {
        step_at(&mut state, at_step(now, step));
    }
    assert_eq!(state.step_index(), GRID_STEPS - 1);

    assert_eq!(
        step_at(&mut state, at_step(now, GRID_STEPS as u64)),
        vec![[0x90, 60, 100]]
    );
    assert_eq!(state.step_index(), 0);
}

/// 先読みは送信だけ先行させる。表示位置は締切が来るまで進めない。
#[test]
fn the_playhead_does_not_run_ahead_of_the_lookahead() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);

    let scheduled = state.poll_steps(now, STEP_INTERVAL * 3);

    // 0〜3ステップ目まで組み立て済みでも、鳴っているのはまだ0ステップ目。
    assert!(!scheduled.is_empty());
    assert_eq!(state.step_index(), 0);

    state.poll_steps(at_step(now, 2), STEP_INTERVAL * 3);
    assert_eq!(state.step_index(), 2);
}

#[test]
fn take_reset_messages_silences_everything_and_stops_the_clock() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].base_note = 60;
    state.instances[0].pattern.draw_span(0, 3);
    state.instances[1].base_note = 72;
    state.instances[1].pattern.draw_span(0, 3);
    state.start(now);
    step_at(&mut state, now);

    let reset = state.take_reset_messages();
    assert_eq!(
        reset.iter().map(|event| event.message).collect::<Vec<_>>(),
        vec![[0x80, 60, 0], [0x80, 72, 0]]
    );
    assert_eq!(
        reset
            .iter()
            .map(|event| event.instance_id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(!state.is_running());
    assert_eq!(state.step_index(), 0);
    assert!(step_at(&mut state, at_step(now, 1)).is_empty());
}

#[test]
fn restarting_replays_from_the_first_step() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.instances[0].pattern.draw_span(0, 0);
    state.start(now);
    step_at(&mut state, now);
    step_at(&mut state, at_step(now, 1));
    assert_eq!(state.step_index(), 1);

    let restarted = at_step(now, 10);
    state.start(restarted);
    assert_eq!(step_at(&mut state, restarted), vec![[0x90, 60, 100]]);
    assert_eq!(state.step_index(), 0);
}

#[test]
fn patches_carry_the_instance_id_of_the_playing_bank() {
    let mut state = GridState::default();
    state.instances[0].patch = Some("first/Patch.fxp".to_string());
    state.instances[1].patch = Some("second/Patch.fxp".to_string());
    assert_eq!(
        state.patches().take(2).collect::<Vec<_>>(),
        vec![(0, Some("first/Patch.fxp")), (1, Some("second/Patch.fxp")),],
        "行順に、いま鳴っている bank の instance ID が付く"
    );
}
