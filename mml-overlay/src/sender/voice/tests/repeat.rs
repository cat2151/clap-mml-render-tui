//! repeat は「張り直さずに継ぎ足す」。それをサーバーへ送ったコマンド列で確かめる。
//!
//! 継ぎ目の有無は最終的には耳でしか分からないが、**張り直していないこと**（＝
//! `BeginTimeline` が 1 回きりで、同じ `timeline_id` へ未来の秒が積まれ続けること）は
//! ここで機械的に固定できる。張り直しが混ざれば必ずここが落ちる。

use super::*;

/// 4 音・1 周 1.0 秒の、鳴らし続ける行。
fn looping() -> LineProgram {
    let mut program = line(4);
    program.repeat = true;
    assert_eq!(program.performance.loop_seconds, 1.0);
    program
}

/// 何周ぶん積まれたか。1 周 4 イベントなので件数で数える。
fn laps(sink: &FakeSink) -> usize {
    sink.timeline_events().len() / 4
}

/// 張り直したら継ぎ目が出る。何周積んでも `BeginTimeline` は最初の 1 回だけ。
#[test]
fn the_timeline_is_begun_once_no_matter_how_many_laps() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &looping());
    for step in 1..=8 {
        voice.pump_repeat(&sink, origin + Duration::from_secs(step));
    }

    assert_eq!(sink.count(&Sent::BeginTimeline), 1);
    // 8 秒経過ぶん + 先読み 4 秒ぶんは積まれている。
    assert!(laps(&sink) >= 12, "laps={}", laps(&sink));
}

/// 継ぎ足しは必ず未来へ伸びる。同じ timeline のまま、時刻が戻らない。
#[test]
fn every_lap_extends_the_same_timeline_into_the_future() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &looping());
    for step in 1..=5 {
        voice.pump_repeat(&sink, origin + Duration::from_secs(step));
    }

    let events = sink.timeline_events();
    let timeline_id = events[0].timeline_id;
    assert!(events.iter().all(|event| event.timeline_id == timeline_id));
    assert!(
        events
            .windows(2)
            .all(|pair| pair[1].timeline_seconds > pair[0].timeline_seconds),
        "timeline_seconds must be strictly increasing"
    );
    // 周と周の間隔は 1 周の長さそのもの。
    let starts: Vec<f64> = events
        .iter()
        .step_by(4)
        .map(|event| event.timeline_seconds)
        .collect();
    for pair in starts.windows(2) {
        assert!((pair[1] - pair[0] - 1.0).abs() < 1e-9, "starts={starts:?}");
    }
}

/// 先読みが足りているうちは何も送らない。積み過ぎてサーバーのキューを埋めない。
#[test]
fn nothing_is_sent_while_the_lookahead_is_still_full() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();
    voice.play_line(&sink, &looping());
    let after_play = sink.take();

    voice.pump_repeat(&sink, origin);

    assert!(sink.sent().is_empty());
    // 最初の積み込みだけで 4 秒ぶんが埋まっている。
    assert_eq!(after_play.len(), 1 + 4);
    assert_eq!(after_play[0], Sent::BeginTimeline);
}

/// repeat OFF は今までどおり。1 回積んで終わりで、pump しても何も起きない。
#[test]
fn without_repeat_the_command_sequence_is_unchanged() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &line(3));
    voice.pump_repeat(&sink, origin + Duration::from_secs(30));

    assert_eq!(
        sink.sent(),
        vec![Sent::BeginTimeline, Sent::TimelineEvents(3)]
    );
    assert_eq!(voice.next_wake(origin), None);
}

/// 1 周が短すぎる行（1 音だけの行など）は繰り返さない。何千周ぶんも積んでしまうため。
#[test]
fn a_line_that_is_too_short_falls_back_to_playing_once() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();
    let mut program = line(1);
    program.repeat = true;
    program.performance.loop_seconds = 0.0;

    voice.play_line(&sink, &program);
    voice.pump_repeat(&sink, origin + Duration::from_secs(30));

    assert_eq!(
        sink.sent(),
        vec![Sent::BeginTimeline, Sent::TimelineEvents(1)]
    );
}

/// 止めたら継ぎ足しも終わる。ここが残ると、誰も鳴らしていないつもりのまま音が続く。
#[test]
fn stopping_ends_the_loop_for_good() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();
    voice.play_line(&sink, &looping());
    voice.stop(&sink, "test");
    sink.take();

    voice.pump_repeat(&sink, origin + Duration::from_secs(30));

    assert!(sink.sent().is_empty());
    assert_eq!(voice.next_wake(origin + Duration::from_secs(30)), None);
}

/// 新しい行はループごと差し替える。前のループが裏で鳴り続けない。
#[test]
fn a_new_line_replaces_the_running_loop() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();
    voice.play_line(&sink, &looping());
    let first = sink.timeline_events()[0].timeline_id;

    voice.play_line(&sink, &line(3));
    sink.take();
    voice.pump_repeat(&sink, origin + Duration::from_secs(30));

    assert!(sink.sent().is_empty());
    let last = sink.timeline_events().last().unwrap().timeline_id;
    assert_ne!(last, first);
}

/// worker はこの待ちで寝る。repeat が走っている間だけ起こされる。
#[test]
fn the_worker_is_woken_up_for_the_next_lap() {
    let sink = FakeSink::default();
    let mut voice = voice();
    let origin = Instant::now();

    voice.play_line(&sink, &looping());
    let (wake, wait) = voice
        .next_wake(origin)
        .expect("a loop must schedule a wake");

    assert_eq!(wake, Wake::Repeat);
    // 4 秒ぶん積んであるので、遅くとも 1 周ぶんの後には起きる。
    assert!(wait <= Duration::from_secs(1), "wait={wait:?}");
}
