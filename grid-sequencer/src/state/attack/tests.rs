use std::time::{Duration, Instant};

use cmrt_arpeggiator::{generate_bass_line, BassPattern};
use cmrt_chord::ChordVoicing;

use super::super::SCHEDULE_GUARD;
use super::*;
use crate::{step_offset, ChordPlayback, GridScheduledMessage, BASS_ROW};

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

/// bass 付きの C。bass 行は auto voicing を通した進行でしか鳴らない。
fn c_major() -> ChordPlayback {
    ChordPlayback::from_voicings(
        "C",
        "I".to_string(),
        vec![ChordVoicing {
            bass: Some(48),
            notes: vec![60, 64, 67],
        }],
    )
    .unwrap()
}

/// 和音・bass（Whole）・4 voice・Single の 4 行を chord mode で `steps` 列ぶん進めた state。
fn chorded_state_after(now: Instant, steps: u64) -> GridState {
    let mut state = GridState::silent_with_instance_count(4);
    state.set_chord(Some(c_major()), now);
    assert!(state.apply_bass_line(&generate_bass_line(BassPattern::Whole, GRID_STEPS)));
    state.start(now);
    advance(&mut state, now, steps);
    state
}

fn sounding_of(state: &GridState, instance: usize) -> Vec<SoundingNote> {
    state
        .sounding
        .iter()
        .copied()
        .filter(|note| match note.owner {
            SoundOwner::ChordInstance { instance: owner } => owner == instance,
            SoundOwner::Lane(address) => address.instance == instance,
        })
        .collect()
}

fn targets(messages: &[GridScheduledMessage]) -> Vec<(u8, u8, u8)> {
    messages
        .iter()
        .map(|scheduled| {
            (
                scheduled.message[0],
                scheduled.instance_id,
                scheduled.message[1],
            )
        })
        .collect()
}

#[test]
fn a_reattack_restarts_every_voice_of_the_chord_and_keeps_its_remaining_steps() {
    let now = Instant::now();
    let mut state = chorded_state_after(now, 6);
    assert_eq!(state.schedule_index, 5);
    let before = sounding_of(&state, CHORD_ROW);
    assert_eq!(before.len(), 3);

    let messages = state.reattack_instance_now(CHORD_ROW, now + step_offset(5));

    assert_eq!(
        targets(&messages),
        [
            (NOTE_OFF, 0, 60),
            (NOTE_OFF, 0, 64),
            (NOTE_OFF, 0, 67),
            (NOTE_ON, 0, 60),
            (NOTE_ON, 0, 64),
            (NOTE_ON, 0, 67),
        ]
    );
    assert!(messages
        .iter()
        .all(|scheduled| scheduled.ahead == SCHEDULE_GUARD));
    assert_eq!(sounding_of(&state, CHORD_ROW), before);
}

#[test]
fn a_reattack_of_a_whole_bass_restarts_the_root_with_its_remaining_length() {
    let now = Instant::now();
    let mut state = chorded_state_after(now, 6);

    let messages = state.reattack_instance_now(BASS_ROW, now + step_offset(5));

    assert_eq!(targets(&messages), [(NOTE_OFF, 1, 48), (NOTE_ON, 1, 48)]);
    let sounding = sounding_of(&state, BASS_ROW);
    assert_eq!(sounding.len(), 1);
    assert_eq!(
        sounding[0].remaining_steps,
        (GRID_STEPS - 5) as u8,
        "進めた分だけ減った残りをそのまま引き継ぐ"
    );
    // 次の小節頭で切れ、そこで譜面の step 0 の Attack が鳴る。
    let offs = (6..=GRID_STEPS as u64)
        .map(|step| {
            let messages = state.poll_steps(now + step_offset(step), Duration::ZERO);
            messages
                .iter()
                .filter(|scheduled| scheduled.instance_id == 1 && scheduled.message[0] == NOTE_OFF)
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
fn a_reattack_after_a_bank_switch_stops_the_old_instance_and_starts_the_new_one() {
    let now = Instant::now();
    let mut state = chorded_state_after(now, 6);
    let instances = state.instances().to_vec();
    state.stage_next_cycle(instances, c_major());
    state.mark_pending_ready();
    assert!(state.commit_pending_cycle());
    assert_eq!(state.bank(), 1);
    assert_eq!(state.instance_id(BASS_ROW), 5);
    assert_eq!(sounding_of(&state, BASS_ROW)[0].instance_id, 1);

    let messages = state.reattack_instance_now(BASS_ROW, now + step_offset(5));

    assert_eq!(targets(&messages), [(NOTE_OFF, 1, 48), (NOTE_ON, 5, 48)]);
    assert_eq!(
        sounding_of(&state, BASS_ROW)[0].instance_id,
        5,
        "期限が来たときの note off が新しい instance へ届く"
    );
}

#[test]
fn a_reattack_of_an_instance_with_nothing_sounding_is_empty() {
    let now = Instant::now();
    let mut state = chorded_state_after(now, 6);
    // Single 行の譜面は空なので、何も鳴っていない。
    assert!(sounding_of(&state, 3).is_empty());

    assert!(state.reattack_instance_now(3, now).is_empty());
    assert_eq!(state.sounding.len(), 4, "他の行の音には触らない");
}

#[test]
fn a_reattack_is_ignored_while_the_clock_is_stopped() {
    let mut state = GridState::with_row_count(1);
    assert!(state
        .reattack_instance_now(CHORD_ROW, Instant::now())
        .is_empty());
}
