//! 行の演奏経路が、低い key switch の note と「同時刻で key switch が演奏音より前」の
//! 並びを崩さずに timeline へ積むこと。
//!
//! sampler の key switch はラッチ式で、同時刻でも演奏音より後に届くと 1 音遅れて効く。

use super::*;
use crate::sender::RecordingSink;

const SUS_DOWN: u8 = 17;
const HAMMER_ON: u8 = 26;
const PULL_OFF: u8 = 25;

fn event(seconds: f64, message: [u8; 3]) -> cmrt_chord::TimedMidiEvent {
    cmrt_chord::TimedMidiEvent { seconds, message }
}

#[test]
fn low_key_switch_notes_keep_their_place_before_the_played_note_at_the_same_time() {
    let events = vec![
        event(0.0, [NOTE_ON, SUS_DOWN, 127]),
        event(0.0, [NOTE_ON, 40, 100]),
        event(0.5, [NOTE_OFF, SUS_DOWN, 0]),
        event(0.5, [NOTE_OFF, 40, 0]),
        event(0.5, [NOTE_ON, HAMMER_ON, 127]),
        event(0.5, [NOTE_ON, 42, 100]),
        event(1.0, [NOTE_OFF, HAMMER_ON, 0]),
        event(1.0, [NOTE_OFF, 42, 0]),
        event(1.0, [NOTE_ON, PULL_OFF, 127]),
        event(1.0, [NOTE_ON, 40, 100]),
        event(1.5, [NOTE_OFF, PULL_OFF, 0]),
        event(1.5, [NOTE_OFF, 40, 0]),
    ];
    let sink = Arc::new(RecordingSink::default());
    let sender = MmlOverlaySender::with_recording_sink(Arc::clone(&sink), 48_000.0);

    sender.play_line(
        LivePatch::new(Some("keyswitch.sfz")),
        LineProgram::once(LinePerformance {
            events: events.clone(),
            loop_seconds: 1.5,
        }),
    );
    wait_until(|| sink.timeline_events().len() >= events.len());

    let sent = sink.timeline_events();
    assert_eq!(
        sent.iter().map(|event| event.message).collect::<Vec<_>>(),
        events.iter().map(|event| event.message).collect::<Vec<_>>(),
    );
    let head = sent[0].timeline_seconds;
    for (sent, source) in sent.iter().zip(&events) {
        assert!(
            (sent.timeline_seconds - head - source.seconds).abs() < 1e-9,
            "{sent:?} は {source:?} と同じ相対時刻でない"
        );
    }
}
