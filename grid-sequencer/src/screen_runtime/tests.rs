use std::time::Duration;

use super::*;
use crate::LOOKAHEAD;
use crate::STEP_INTERVAL;

const TIMELINE_ID: u64 = 42;

fn scheduled(ahead: Duration, count: usize) -> Vec<GridScheduledMessage> {
    (0..count)
        .map(|index| GridScheduledMessage {
            instance_id: index as u8 % 16,
            ahead,
            timeline_seconds: ahead.as_secs_f64(),
            message: [0x90, 60 + index as u8 % 64, 100],
        })
        .collect()
}

#[test]
fn one_step_becomes_one_batch_with_a_shared_offset() {
    let mut items = scheduled(Duration::ZERO, 2);
    items.extend(scheduled(STEP_INTERVAL, 3));

    let batches = batches(&items, TIMELINE_ID);

    assert_eq!(batches.len(), 1);
    let times = batches[0]
        .iter()
        .map(|event| event.timeline_seconds)
        .collect::<Vec<_>>();
    assert_eq!(
        times,
        vec![
            0.0,
            0.0,
            STEP_INTERVAL.as_secs_f64(),
            STEP_INTERVAL.as_secs_f64(),
            STEP_INTERVAL.as_secs_f64()
        ]
    );
    assert!(batches[0]
        .iter()
        .all(|event| event.timeline_id == TIMELINE_ID));
}

/// スロット容量を超えるときだけバッチを切る。サーバーは受信時の live 位置を基準に
/// offset を解釈するため、同じステップがバッチを跨ぐと頭がばらける。
#[test]
fn a_step_is_never_split_across_batches() {
    let mut items = scheduled(Duration::ZERO, MAX_MIDI_MESSAGES - 1);
    items.extend(scheduled(STEP_INTERVAL, 4));

    let batches = batches(&items, TIMELINE_ID);

    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), MAX_MIDI_MESSAGES - 1);
    assert_eq!(batches[1].len(), 4);
    assert!(batches[0].iter().all(|event| event.timeline_seconds == 0.0));
    assert!(batches[1]
        .iter()
        .all(|event| event.timeline_seconds == STEP_INTERVAL.as_secs_f64()));
}

#[test]
fn everything_fits_into_one_batch_while_it_is_under_the_slot_capacity() {
    let items = scheduled(Duration::ZERO, MAX_MIDI_MESSAGES);

    let batches = batches(&items, TIMELINE_ID);

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].len(), MAX_MIDI_MESSAGES);
}

#[test]
fn nothing_scheduled_sends_nothing() {
    assert!(batches(&[], TIMELINE_ID).is_empty());
}

#[test]
fn sender_delay_does_not_change_absolute_event_time() {
    let mut early = scheduled(Duration::from_millis(200), 1);
    early[0].timeline_seconds = 12.5;
    let mut late = early.clone();
    late[0].ahead = Duration::ZERO;

    assert_eq!(
        batches(&early, TIMELINE_ID)[0][0]
            .timeline_seconds
            .to_bits(),
        batches(&late, TIMELINE_ID)[0][0].timeline_seconds.to_bits()
    );
}

#[test]
fn lookahead_covers_the_next_buffer_level() {
    let screen = GridSequencerScreen::new(None);
    let at_x2 = screen.scheduling_lookahead(2);
    let next_lead = Duration::from_secs_f64(screen.buffer_frames as f64 * 4.0 / screen.sample_rate);
    let required = next_lead * 2 + LOOKAHEAD + Duration::from_millis(50);
    assert!(at_x2 + Duration::from_nanos(1) >= required);
    assert!(screen.scheduling_lookahead(16) > at_x2);
}
