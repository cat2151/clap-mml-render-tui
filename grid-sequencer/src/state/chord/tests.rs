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

fn g_major() -> ChordPlayback {
    ChordPlayback::new("G", "V".to_string(), vec![vec![67, 71, 74]]).unwrap()
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
fn entering_the_last_bar_raises_the_preload_signal_once() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);
    // 1コード目（進行の最後ではない）を鳴らしている間は立たない。
    assert!(!state.take_preload_due());

    // 2コード目 = 最終小節へ入ったところで立つ。
    for step in 1..=GRID_STEPS as u64 {
        step_at(&mut state, at_step(now, step));
    }
    assert!(state.take_preload_due());
    assert!(!state.take_preload_due(), "合図は取ったら降ろす");
    assert_eq!(state.chord().unwrap().index(), 1);
}

/// ダブルバッファの肝。1周し終えた小節の頭で、待機 bank の grid と進行へ丸ごと
/// 差し替わり、**その場で**新しい和音が鳴る（旧実装のような1小節の無音を挟まない）。
#[test]
fn completing_the_progression_swaps_the_ready_bank_and_keeps_playing() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, g_major());
    state.mark_pending_ready();

    // 2コードぶん（= 進行1周）進める。
    let mut wrapped = Vec::new();
    for step in 1..=(GRID_STEPS as u64 * 2) {
        wrapped = step_at(&mut state, at_step(now, step));
    }

    assert_eq!(state.bank(), 1, "待機 bank へ移る");
    assert_eq!(state.chord().unwrap().degrees(), "V");
    assert_eq!(
        messages(&wrapped),
        vec![
            [0x80, 65, 0],
            [0x80, 69, 0],
            [0x80, 72, 0],
            [0x90, 67, 100],
            [0x90, 71, 100],
            [0x90, 74, 100],
        ],
        "最後のコードを切ってから、間を空けずに新しい和音を鳴らす"
    );
    // 新しい和音は差し替え後の bank の instance へ出す。
    let attacks = wrapped
        .iter()
        .filter(|item| item.message[0] == 0x90)
        .collect::<Vec<_>>();
    assert!(attacks
        .iter()
        .all(|item| item.instance_id == state.instance_id(CHORD_ROW)));
}

/// 先読みが間に合わなかったら差し替えを見送り、今の grid のまま次の周へ入る。
/// 音を止めないことを最優先する。
#[test]
fn an_unfinished_preload_keeps_the_current_bank() {
    let now = Instant::now();
    let mut state = GridState::default();
    state.set_chord(Some(c_major_then_f_major()), now);
    state.start(now);
    step_at(&mut state, now);
    let rows = state.rows().to_vec();
    state.stage_next_cycle(rows, g_major());
    // `mark_pending_ready()` を呼ばない = ロードがまだ終わっていない。

    for step in 1..=(GRID_STEPS as u64 * 2) {
        step_at(&mut state, at_step(now, step));
    }

    assert_eq!(state.bank(), 0, "bank は動かさない");
    assert_eq!(state.chord().unwrap().degrees(), "I-IV");
    assert!(state.has_pending_cycle(), "差し替え待ちは次の周へ持ち越す");
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
