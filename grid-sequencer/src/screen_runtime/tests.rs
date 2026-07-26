use std::time::Duration;

use super::*;
use crate::STEP_INTERVAL;

const SAMPLE_RATE: f64 = 48_000.0;

fn scheduled(ahead: Duration, count: usize) -> Vec<GridScheduledMessage> {
    (0..count)
        .map(|index| GridScheduledMessage {
            ahead,
            message: [0x90, 60 + index as u8 % 64, 100],
        })
        .collect()
}

#[test]
fn one_step_becomes_one_batch_with_a_shared_offset() {
    let mut items = scheduled(Duration::ZERO, 2);
    items.extend(scheduled(STEP_INTERVAL, 3));

    let batches = batches(&items, SAMPLE_RATE);

    assert_eq!(batches.len(), 1);
    let offsets = batches[0]
        .iter()
        .map(|(offset, _)| *offset)
        .collect::<Vec<_>>();
    // 48000 * 60 / (130*4) = 5538 サンプル。
    assert_eq!(offsets, vec![0, 0, 5538, 5538, 5538]);
}

/// スロット容量を超えるときだけバッチを切る。サーバーは受信時の live 位置を基準に
/// offset を解釈するため、同じステップがバッチを跨ぐと頭がばらける。
#[test]
fn a_step_is_never_split_across_batches() {
    let mut items = scheduled(Duration::ZERO, MAX_MIDI_MESSAGES - 1);
    items.extend(scheduled(STEP_INTERVAL, 4));

    let batches = batches(&items, SAMPLE_RATE);

    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), MAX_MIDI_MESSAGES - 1);
    assert_eq!(batches[1].len(), 4);
    assert!(batches[0].iter().all(|(offset, _)| *offset == 0));
    assert!(batches[1].iter().all(|(offset, _)| *offset == 5538));
}

#[test]
fn everything_fits_into_one_batch_while_it_is_under_the_slot_capacity() {
    let items = scheduled(Duration::ZERO, MAX_MIDI_MESSAGES);

    let batches = batches(&items, SAMPLE_RATE);

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].len(), MAX_MIDI_MESSAGES);
}

#[test]
fn nothing_scheduled_sends_nothing() {
    assert!(batches(&[], SAMPLE_RATE).is_empty());
}
