use super::*;

fn events(count: usize) -> Vec<TimedMidiEvent> {
    (0..count)
        .map(|index| TimedMidiEvent {
            seconds: index as f64 * 0.25,
            message: [0x90, 60, 127],
        })
        .collect()
}

#[test]
fn every_event_is_pushed_forward_by_the_lookahead() {
    let batches = timeline_batches(&events(2), 7);

    assert_eq!(batches[0][0].timeline_seconds, LOOKAHEAD_SECONDS);
    assert_eq!(batches[0][1].timeline_seconds, LOOKAHEAD_SECONDS + 0.25);
    assert_eq!(batches[0][0].timeline_id, 7);
    assert_eq!(batches[0][0].instance_id, MML_OVERLAY_INSTANCE);
}

/// サーバーは 1 バッチに載る数を超えるとバッチ全体を弾く。必ず切り分けて送ること。
#[test]
fn a_long_phrase_is_split_into_batches_the_server_accepts() {
    let batches = timeline_batches(&events(MAX_MIDI_MESSAGES + 1), 1);

    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), MAX_MIDI_MESSAGES);
    assert_eq!(batches[1].len(), 1);
}

#[test]
fn a_phrase_longer_than_the_queue_is_truncated() {
    let batches = timeline_batches(&events(MAX_LINE_EVENTS + 100), 1);

    let total = batches.iter().map(Vec::len).sum::<usize>();
    assert_eq!(total, MAX_LINE_EVENTS);
}

/// 負の秒はサーバーに弾かれる。SMF 由来の値がずれても送る側で潰しておく。
#[test]
fn a_negative_time_is_clamped_to_the_start() {
    let batches = timeline_batches(
        &[TimedMidiEvent {
            seconds: -1.0,
            message: [0x90, 60, 127],
        }],
        1,
    );

    assert_eq!(batches[0][0].timeline_seconds, LOOKAHEAD_SECONDS);
}

#[test]
fn each_line_gets_a_fresh_non_zero_timeline_id() {
    assert_eq!(LinePlayback::new(48_000.0).next_timeline_id, 1);
    assert_eq!(advance_timeline_id(1), 2);
    // 一周しても 0 は使わない。サーバーが無効値として弾くため。
    assert_eq!(advance_timeline_id(TimelineId::MAX), 1);
}

#[test]
fn a_new_playback_has_nothing_to_stop() {
    assert!(!LinePlayback::new(48_000.0).is_active());
}
