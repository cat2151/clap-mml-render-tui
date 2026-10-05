//! 停止を同じ command に含めた行は、load 待ちの間も古い予約を残さない。
use super::*;

fn replacement(patch: &str) -> SenderCommandKind {
    SenderCommandKind::PlayLine {
        patch: LivePatch::new(Some(patch)),
        program: LineProgram::once(LinePerformance {
            events: vec![
                cmrt_chord::TimedMidiEvent {
                    seconds: 0.0,
                    message: [NOTE_ON, 36, 127],
                },
                cmrt_chord::TimedMidiEvent {
                    seconds: 3.875,
                    message: [NOTE_OFF, 36, 0],
                },
            ],
            loop_seconds: 3.875,
        }),
        stop_before_prepare: true,
    }
}

#[test]
fn replacement_stops_before_loading_and_a_newer_stop_cancels_the_loaded_line() {
    let sink = Arc::new(FakeSink {
        prepare_delay: Duration::from_millis(100),
        ..Default::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(4.0, false));
    wait_until(|| harness.status.lock().unwrap().line_playback().is_some());
    harness.send(2, replacement("new.sfz"));
    wait_until(|| {
        let status = harness.status.lock().unwrap();
        status.command_id() == 2 && status.is_loading()
    });
    assert_eq!(
        sink.stops(),
        1,
        "load starts only after clearing old timeline"
    );
    assert_eq!(
        sink.begins(),
        1,
        "loading cannot play through the previous kit"
    );
    harness.send(3, SenderCommandKind::Stop);
    wait_until(|| harness.status.lock().unwrap().command_id() == 3);
    assert_eq!(sink.begins(), 1, "superseded load cannot enqueue notes");
    assert!(harness.status.lock().unwrap().line_playback().is_none());
}
