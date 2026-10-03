//! 長い行を送るときの、共有メモリのコマンド枠とサーバーの待ち行列の扱い。
//!
//! 枠が満杯なら待って送り直す。サーバーが待ち行列からあふれた分を捨てたら、次の停止を
//! 全音停止にする。どちらも送ったコマンド列とログ行で確かめる。

use std::time::Instant;

use super::*;
use crate::sender::line_playback::{MAX_LINE_EVENTS, QUEUE_FULL_WAIT_LIMIT};
use crate::test_log;

#[test]
fn a_full_command_queue_is_retried_without_losing_or_repeating_a_batch() {
    let sink = FakeSink::default();
    sink.queue_full_replies.set(3);
    let mut voice = voice();

    assert!(voice.play_line(&sink, &line(300)));

    assert_eq!(
        sink.take(),
        vec![
            Sent::BeginTimeline,
            Sent::TimelineEvents(128),
            Sent::TimelineEvents(128),
            Sent::TimelineEvents(44),
        ]
    );
    let seconds = sink
        .timeline_events()
        .iter()
        .map(|event| event.timeline_seconds)
        .collect::<Vec<_>>();
    assert_eq!(seconds.len(), 300);
    assert!(seconds.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn a_queue_that_never_frees_up_fails_the_line_after_the_wait_limit() {
    test_log::install();
    let sink = FakeSink::default();
    sink.queue_full_replies.set(usize::MAX);
    let mut voice = voice();
    let started_at = Instant::now();

    assert!(!voice.play_line(&sink, &line(4)));

    assert!(started_at.elapsed() >= QUEUE_FULL_WAIT_LIMIT);
    assert!(sink.timeline_events().is_empty());
    assert!(!test_log::lines_containing(&[
        "action=mml-overlay-line-send event=error",
        "shared-memory MIDI queue is full",
    ])
    .is_empty());
    // 積めなかった行の次の停止は、全音停止になる。
    sink.take();
    voice.stop(&sink, "test");
    assert_eq!(sink.take(), vec![Sent::StopAll]);
}

#[test]
fn events_dropped_by_the_server_turn_the_next_stop_into_a_full_stop() {
    test_log::install();
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.begin_command(71_001);
    sink.dropped_total.set(10);
    assert!(voice.play_line(&sink, &line(4)));
    sink.dropped_total.set(15);
    sink.take();

    voice.begin_command(71_002);
    assert!(voice.play_line(&sink, &line(4)));

    assert_eq!(
        sink.take(),
        vec![Sent::StopAll, Sent::BeginTimeline, Sent::TimelineEvents(4)]
    );
    assert_eq!(
        test_log::lines_containing(&["event=server-dropped", "command_id=71002"]),
        vec!["mml-overlay: action=mml-overlay-line-send event=server-dropped command_id=71002 count=5"]
    );
}

#[test]
fn without_drops_the_next_line_only_rebuilds_the_timeline() {
    test_log::install();
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.begin_command(72_001);
    sink.dropped_total.set(10);
    assert!(voice.play_line(&sink, &line(4)));
    sink.take();

    voice.begin_command(72_002);
    assert!(voice.play_line(&sink, &line(4)));

    assert_eq!(
        sink.take(),
        vec![Sent::BeginTimeline, Sent::TimelineEvents(4)]
    );
    assert!(test_log::lines_containing(&["event=server-dropped", "command_id=72002"]).is_empty());
}

#[test]
fn a_line_longer_than_the_limit_is_still_truncated() {
    test_log::install();
    let sink = FakeSink::default();
    let mut voice = voice();
    let total = MAX_LINE_EVENTS + 7;

    assert!(!voice.play_line(&sink, &line(total)));

    assert_eq!(sink.timeline_events().len(), MAX_LINE_EVENTS);
    assert!(!test_log::lines_containing(&[
        "event=truncated",
        &format!("total={total} kept={MAX_LINE_EVENTS}"),
    ])
    .is_empty());
}
