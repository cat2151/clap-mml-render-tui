//! 「鳴らす前に必ず止まる」を、サーバーへ送ったコマンド列で確かめる。
//!
//! 鳴りっぱなしの正体は「サーバーへ 1 つもコマンドが飛ばない経路」だった。
//! `Voice` の内部状態ではなく**送信の記録**を見ないと、その穴は塞げたか分からない。

use std::cell::RefCell;

use cmrt_realtime_play::{LiveTimelineConfig, TimelineMidiEvent};

use super::*;
use crate::sender::sink::SinkResult;
use crate::{NOTE_OFF, NOTE_ON};

/// サーバーへ飛んだコマンド。
#[derive(Clone, Debug, PartialEq)]
enum Sent {
    Prepare(Option<String>),
    Midi(Vec<[u8; 3]>),
    StopAll,
    BeginTimeline,
    TimelineEvents(usize),
}

#[derive(Default)]
struct FakeSink {
    sent: RefCell<Vec<Sent>>,
    /// 生 MIDI の送信を失敗させる。
    midi_fails: bool,
    /// timeline を張るのを失敗させる。
    begin_fails: bool,
}

impl FakeSink {
    fn sent(&self) -> Vec<Sent> {
        self.sent.borrow().clone()
    }

    fn take(&self) -> Vec<Sent> {
        self.sent.borrow_mut().drain(..).collect()
    }

    fn push(&self, sent: Sent) {
        self.sent.borrow_mut().push(sent);
    }
}

impl SoundSink for FakeSink {
    fn prepare_patch(&self, patch: Option<&str>) -> SinkResult {
        self.push(Sent::Prepare(patch.map(str::to_string)));
        Ok(())
    }

    fn send_midi(&self, messages: &[[u8; 3]]) -> SinkResult {
        self.push(Sent::Midi(messages.to_vec()));
        if self.midi_fails {
            return Err("midi failed".to_string());
        }
        Ok(())
    }

    fn stop_all(&self) -> SinkResult {
        self.push(Sent::StopAll);
        Ok(())
    }

    fn begin_timeline(&self, _config: LiveTimelineConfig) -> SinkResult {
        self.push(Sent::BeginTimeline);
        if self.begin_fails {
            return Err("begin failed".to_string());
        }
        Ok(())
    }

    fn send_timeline_events(&self, events: &[TimelineMidiEvent]) -> SinkResult {
        self.push(Sent::TimelineEvents(events.len()));
        Ok(())
    }
}

fn voice() -> Voice {
    Voice::new(48_000.0)
}

fn note_on(pitch: u8) -> [u8; 3] {
    [NOTE_ON, pitch, 127]
}

fn note_off(pitch: u8) -> [u8; 3] {
    [NOTE_OFF, pitch, 0]
}

fn line(count: usize) -> Vec<TimedMidiEvent> {
    (0..count)
        .map(|index| TimedMidiEvent {
            seconds: index as f64 * 0.25,
            message: note_on(60),
        })
        .collect()
}

/// 鳴っていないのに止めに行かない。無駄なリセットは音を切ってしまう。
#[test]
fn stopping_silence_sends_nothing() {
    let sink = FakeSink::default();

    voice().stop(&sink, "test");

    assert!(sink.sent().is_empty());
}

#[test]
fn the_next_note_stops_the_previous_one_first() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)]);
    sink.take();

    voice.play_notes(&sink, &[note_on(62)]);

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(vec![note_off(60)]),
            Sent::Midi(vec![note_on(62)]),
        ]
    );
}

/// これが直したかったバグ。空行へ移ると `play_line` が空で呼ばれる。以前は
/// timeline 側が「自分は鳴らしていない」と早期 return し、打鍵の note off ごと
/// 握り潰していたので音が永久に残った。
#[test]
fn an_empty_line_still_stops_the_typed_note() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)]);
    sink.take();

    voice.play_line(&sink, &[]);

    assert_eq!(sink.sent(), vec![Sent::Midi(vec![note_off(60)])]);
}

/// 行を鳴らす経路でも同じ。timeline を張る前に打鍵の音を止める。
#[test]
fn playing_a_line_stops_the_typed_note_before_the_timeline() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)]);
    sink.take();

    voice.play_line(&sink, &line(3));

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(vec![note_off(60)]),
            Sent::BeginTimeline,
            Sent::TimelineEvents(3),
        ]
    );
}

/// timeline の音は note off では止まらないので、音源リセットで止める。
#[test]
fn a_line_is_stopped_by_resetting_the_instrument() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_line(&sink, &line(3));
    sink.take();

    voice.stop(&sink, "test");

    assert_eq!(sink.sent(), vec![Sent::StopAll]);
}

/// 行が鳴っている最中の打鍵も、まず音源リセットで行を止める。
#[test]
fn typing_during_a_line_resets_the_instrument_first() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_line(&sink, &line(3));
    sink.take();

    voice.play_notes(&sink, &[note_on(60)]);

    assert_eq!(
        sink.sent(),
        vec![Sent::StopAll, Sent::Midi(vec![note_on(60)])]
    );
}

/// 音色の差し替えも「鳴らす前」と同じ。前の音色の音を引きずらせない。
#[test]
fn preparing_a_patch_stops_what_is_sounding() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)]);
    sink.take();

    voice.prepare(&sink, Some("lead.fxp"));

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(vec![note_off(60)]),
            Sent::Prepare(Some("lead.fxp".to_string())),
        ]
    );
}

/// note off が届かなかったら、音源リセットへ格上げして必ず黙らせる。
#[test]
fn a_failed_note_off_escalates_to_an_instrument_reset() {
    let mut sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)]);
    sink.take();
    sink.midi_fails = true;

    voice.stop(&sink, "test");

    assert_eq!(
        sink.sent(),
        vec![Sent::Midi(vec![note_off(60)]), Sent::StopAll]
    );
}

/// note on の送信に失敗しても、届いていたかもしれない前提で次は音源リセットで止める。
#[test]
fn a_failed_note_on_is_stopped_by_resetting_the_instrument() {
    let mut sink = FakeSink::default();
    let mut voice = voice();
    sink.midi_fails = true;
    voice.play_notes(&sink, &[note_on(60)]);
    sink.midi_fails = false;
    sink.take();

    voice.stop(&sink, "test");

    assert_eq!(sink.sent(), vec![Sent::StopAll]);
}

/// timeline を張れなかったら音は出ていない。次の停止で無駄なリセットを撒かない
/// ……のではなく、張れたかどうかが怪しいので必ずリセットを送る。
#[test]
fn a_failed_timeline_still_gets_stopped() {
    let mut sink = FakeSink::default();
    let mut voice = voice();
    sink.begin_fails = true;
    voice.play_line(&sink, &line(3));
    sink.take();

    voice.stop(&sink, "test");

    assert_eq!(sink.sent(), vec![Sent::StopAll]);
}

/// 音源リセットまで通れば実態は確実に黙る。そのあとは無駄に止めに行かない。
#[test]
fn a_hard_stop_leaves_nothing_to_stop() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_line(&sink, &line(3));
    voice.stop(&sink, "test");
    sink.take();

    voice.stop(&sink, "test");

    assert!(sink.sent().is_empty());
}
