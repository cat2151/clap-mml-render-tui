use std::time::{Duration, Instant};

use super::super::SCHEDULE_GUARD;
use super::*;
use crate::{step_offset, GridScheduledMessage};

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const FIRST_LANE: LaneAddress = LaneAddress {
    instance: 0,
    lane: 0,
};

/// `steps` 列ぶん組み立てて、組み立て位置を `steps - 1` へ進める。
fn advance(state: &mut GridState, start: Instant, steps: u64) -> Vec<GridScheduledMessage> {
    (0..steps)
        .flat_map(|step| state.poll_steps(start + step_offset(step), Duration::ZERO))
        .collect()
}

fn kinds(messages: &[GridScheduledMessage]) -> Vec<u8> {
    messages
        .iter()
        .map(|scheduled| scheduled.message[0])
        .collect()
}

#[test]
fn a_preview_sounds_at_once_and_stops_at_the_bar_end() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.start(now);
    advance(&mut state, now, 6);
    assert_eq!(state.schedule_index, 5);

    let preview = state.preview_lane_now(FIRST_LANE, now + step_offset(5));

    // 譜面には何も無いので、鳴るのはプレビューの1音だけ。
    assert_eq!(kinds(&preview), [NOTE_ON]);
    assert_eq!(preview[0].message[1], 60);
    // 先読み済みの列（step 5）より後ろへ置く。
    assert_eq!(preview[0].ahead, SCHEDULE_GUARD);

    // step 6〜15 は鳴りっぱなしで、次の小節頭（16列目）で切れる。
    let offs = (6..=GRID_STEPS as u64)
        .map(|step| {
            let messages = state.poll_steps(now + step_offset(step), Duration::ZERO);
            kinds(&messages)
                .iter()
                .filter(|kind| **kind == NOTE_OFF)
                .count()
        })
        .collect::<Vec<_>>();
    assert_eq!(offs.last().copied(), Some(1), "{offs:?}");
    assert!(
        offs[..offs.len() - 1].iter().all(|count| *count == 0),
        "{offs:?}"
    );
}

#[test]
fn a_preview_replaces_the_note_already_sounding_on_that_lane() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.instances[0].pattern.draw_span(0, GRID_STEPS - 1);
    state.start(now);
    advance(&mut state, now, 1);

    let preview = state.preview_lane_now(FIRST_LANE, now);

    // 鳴っていた音を止めてから鳴らし直す。順序が逆だと消し合う。
    assert_eq!(kinds(&preview), [NOTE_OFF, NOTE_ON]);
}

#[test]
fn a_preview_is_ignored_while_the_clock_is_stopped() {
    let mut state = GridState::with_row_count(1);
    assert!(state
        .preview_lane_now(FIRST_LANE, Instant::now())
        .is_empty());
}

#[test]
fn a_preview_of_a_lane_without_a_note_is_ignored() {
    let now = Instant::now();
    let mut state = GridState::with_row_count(1);
    state.start(now);

    // chord OFF では lane 0 しか音高を持たない。
    assert!(state
        .preview_lane_now(LaneAddress::new(0, 9), now)
        .is_empty());
}
