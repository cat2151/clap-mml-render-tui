//! 「鳴らす前に必ず止まる」を、サーバーへ送ったコマンド列で確かめる。
//!
//! 鳴りっぱなしの正体は「サーバーへ 1 つもコマンドが飛ばない経路」だった。
//! `Voice` の内部状態ではなく**送信の記録**を見ないと、その穴は塞げたか分からない。

use std::cell::RefCell;

use cmrt_chord::TimedMidiEvent;
use cmrt_realtime_play::{LiveTimelineConfig, TimelineMidiEvent};

use super::*;
use crate::line_play::{FilterSettings, LinePerformance};
use crate::sender::live_patch::LivePatch;
use crate::sender::sink::SinkResult;
use crate::{NOTE_OFF, NOTE_ON};

/// サーバーへ飛んだコマンド。
#[derive(Clone, Debug, PartialEq)]
enum Sent {
    Prepare(u8, Option<String>),
    StandbyPrepare(u8, Option<String>),
    Midi(u8, Vec<[u8; 3]>),
    StopAll,
    BeginTimeline,
    TimelineEvents(usize),
}

#[derive(Default)]
struct FakeSink {
    sent: RefCell<Vec<Sent>>,
    /// timeline へ積んだイベントそのもの。継ぎ足しの時刻と timeline id を見るために要る
    /// （[`Sent::TimelineEvents`] は件数しか持たない）。
    timeline: RefCell<Vec<TimelineMidiEvent>>,
    prepare_delay: Duration,
    /// 生 MIDI の送信を失敗させる。
    midi_fails: bool,
    /// timeline を張るのを失敗させる。
    begin_fails: bool,
    midi_delay: Duration,
    /// bank を 2 つ持つ（instance 0 と 1 が互いのもう一方の bank）。
    two_banks: bool,
    /// 準備で受け取った effect chain（[`Sent::Prepare`] / [`Sent::StandbyPrepare`] と同じ順）。
    prepared_chains: RefCell<Vec<String>>,
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

    fn record_chain(&self, patch: &LivePatch) {
        self.prepared_chains
            .borrow_mut()
            .push(patch.effect_chain().to_string());
    }

    fn prepared_chains(&self) -> Vec<String> {
        self.prepared_chains.borrow().clone()
    }

    fn timeline_events(&self) -> Vec<TimelineMidiEvent> {
        self.timeline.borrow().clone()
    }

    fn count(&self, kind: &Sent) -> usize {
        self.sent
            .borrow()
            .iter()
            .filter(|sent| *sent == kind)
            .count()
    }
}

impl SoundSink for FakeSink {
    fn prepare_patch(&self, instance_id: u8, patch: &LivePatch) -> SinkResult {
        std::thread::sleep(self.prepare_delay);
        self.push(Sent::Prepare(
            instance_id,
            patch.patch().map(str::to_string),
        ));
        self.record_chain(patch);
        Ok(())
    }

    fn standby_instance_of(&self, instance_id: u8) -> Option<u8> {
        self.two_banks.then_some(instance_id ^ 1)
    }

    fn prepare_standby_patch(&self, instance_id: u8, patch: &LivePatch) -> SinkResult {
        self.push(Sent::StandbyPrepare(
            instance_id,
            patch.patch().map(str::to_string),
        ));
        self.record_chain(patch);
        Ok(())
    }

    fn send_midi(&self, instance_id: u8, messages: &[[u8; 3]]) -> SinkResult {
        std::thread::sleep(self.midi_delay);
        self.push(Sent::Midi(instance_id, messages.to_vec()));
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
        self.timeline.borrow_mut().extend_from_slice(events);
        Ok(())
    }
}

fn voice() -> Voice {
    Voice::new(48_000.0, SoundingLines::default())
}

/// chain 無しの音色。
fn patch(name: &str) -> LivePatch {
    LivePatch::new(Some(name))
}

fn note_on(pitch: u8) -> [u8; 3] {
    [NOTE_ON, pitch, 127]
}

fn note_off(pitch: u8) -> [u8; 3] {
    [NOTE_OFF, pitch, 0]
}

/// `count` 個の note on が 0.25 秒おきに並ぶ 1 行。1 回だけ鳴らす指示にする。
fn line(count: usize) -> LineProgram {
    LineProgram::once(LinePerformance {
        events: (0..count)
            .map(|index| TimedMidiEvent {
                seconds: index as f64 * 0.25,
                message: note_on(60),
            })
            .collect(),
        loop_seconds: count as f64 * 0.25,
    })
}

#[test]
fn zero_note_window_is_reported_as_an_audibility_risk() {
    assert_eq!(audibility(Some(0)), "at-risk-zero-window");
    assert_eq!(audibility(Some(1)), "at-risk-short-window");
    assert_eq!(audibility(Some(20)), "at-risk-short-window");
    assert_eq!(audibility(Some(21)), "unverified-nonzero-window");
    assert_eq!(audibility(None), "unverified-no-note-window");
    assert_eq!(optional_ms(Some(0)), "0");
    assert_eq!(optional_ms(None), "unknown");
}

#[test]
fn gate_starts_after_slow_patch_prepare_and_keeps_the_full_note_length() {
    let sink = FakeSink {
        prepare_delay: Duration::from_millis(40),
        ..FakeSink::default()
    };
    let mut voice = voice();
    let gate = Duration::from_secs(2);
    let request_started = Instant::now();

    assert!(voice
        .prepare(&sink, MML_OVERLAY_INSTANCE, &patch("slow.sfz"))
        .is_ok());
    assert!(voice.play_notes(&sink, &[note_on(60)], gate));

    assert!(request_started.elapsed() >= Duration::from_millis(40));
    assert!(voice.gate_wait(Instant::now()).unwrap() > Duration::from_millis(1900));
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
    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));
    sink.take();

    voice.play_notes(&sink, &[note_on(62)], Duration::from_millis(250));

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_off(60)]),
            Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_on(62)]),
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
    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));
    sink.take();

    voice.play_line(&sink, &LineProgram::silent());

    assert_eq!(
        sink.sent(),
        vec![Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_off(60)])]
    );
}

/// 行を鳴らす経路でも同じ。timeline を張る前に打鍵の音を止める。
#[test]
fn playing_a_line_stops_the_typed_note_before_the_timeline() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));
    sink.take();

    voice.play_line(&sink, &line(3));

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_off(60)]),
            Sent::BeginTimeline,
            Sent::TimelineEvents(3),
        ]
    );
}

/// timeline の音はclient側で追わず、server管理の全NoteOffで止める。
#[test]
fn a_line_is_stopped_by_the_server_managed_all_note_off() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_line(&sink, &line(3));
    sink.take();

    voice.stop(&sink, "test");

    assert_eq!(sink.sent(), vec![Sent::StopAll]);
}

/// 行が鳴っている最中の打鍵も、まずserver管理の全NoteOffで行を止める。
#[test]
fn typing_during_a_line_stops_the_timeline_first() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_line(&sink, &line(3));
    sink.take();

    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));

    assert_eq!(
        sink.sent(),
        vec![
            Sent::StopAll,
            Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_on(60)])
        ]
    );
}

/// 音色の差し替えも「鳴らす前」と同じ。前の音色の音を引きずらせない。
#[test]
fn preparing_a_patch_stops_what_is_sounding() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));
    sink.take();

    let _ = voice.prepare(&sink, MML_OVERLAY_INSTANCE, &patch("lead.fxp"));

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_off(60)]),
            Sent::Prepare(MML_OVERLAY_INSTANCE, Some("lead.fxp".to_string())),
        ]
    );
}

/// note off が届かなかったら、server管理の全NoteOffへ格上げして必ず黙らせる。
#[test]
fn a_failed_note_off_escalates_to_the_server_managed_stop() {
    let mut sink = FakeSink::default();
    let mut voice = voice();
    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));
    sink.take();
    sink.midi_fails = true;

    voice.stop(&sink, "test");

    assert_eq!(
        sink.sent(),
        vec![
            Sent::Midi(MML_OVERLAY_INSTANCE, vec![note_off(60)]),
            Sent::StopAll
        ]
    );
}

/// note on の送信に失敗しても、届いていたかもしれない前提で次はserver管理で止める。
#[test]
fn a_failed_note_on_is_stopped_by_the_server_managed_stop() {
    let mut sink = FakeSink::default();
    let mut voice = voice();
    sink.midi_fails = true;
    voice.play_notes(&sink, &[note_on(60)], Duration::from_millis(250));
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

/// server管理の全NoteOffまで通れば実態は確実に黙る。そのあとは無駄に止めに行かない。
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

#[cfg(test)]
mod effect_chain;
mod filters;
mod repeat;

/// 次の行は、張り直す timeline に前の行の停止を任せる。`stop_all` を挟むと server が
/// 出力リングを捨て、前の行の音が release を待たずに段差で切れる。
#[test]
fn the_next_line_is_stopped_by_the_new_timeline_not_by_stop_all() {
    let sink = FakeSink::default();
    let mut voice = voice();
    voice.play_line(&sink, &line(3));
    sink.take();

    voice.play_line(&sink, &line(2));

    assert_eq!(
        sink.sent(),
        vec![Sent::BeginTimeline, Sent::TimelineEvents(2)]
    );
}

/// 音色を変える行は、鳴っている bank を止めずにもう一方の bank へ読み込み、そちらで鳴らす。
/// 同じ instance で読み直すと、読み込みの間 bank の render が止まって無音が挟まる。
#[test]
fn a_line_with_another_patch_loads_on_the_other_bank_while_the_old_one_sounds() {
    let sink = FakeSink {
        two_banks: true,
        ..FakeSink::default()
    };
    let mut voice = voice();
    // 何も鳴っていなければ、その場で読み込む。
    voice.prepare_line(&sink, &patch("a.fxp")).unwrap();
    voice.play_line(&sink, &line(1));
    assert_eq!(sink.take()[0], Sent::Prepare(0, Some("a.fxp".to_string())));

    voice.prepare_line(&sink, &patch("b.fxp")).unwrap();
    voice.play_line(&sink, &line(1));

    assert_eq!(
        sink.take(),
        vec![
            Sent::StandbyPrepare(1, Some("b.fxp".to_string())),
            Sent::BeginTimeline,
            Sent::TimelineEvents(1),
        ]
    );
    assert_eq!(sink.timeline_events().last().unwrap().instance_id, 1);

    // 前の音色は元の bank に残っているので、戻るときは読み込まない。
    voice.prepare_line(&sink, &patch("a.fxp")).unwrap();
    voice.play_line(&sink, &line(1));
    assert_eq!(
        sink.take(),
        vec![Sent::BeginTimeline, Sent::TimelineEvents(1)]
    );
    assert_eq!(sink.timeline_events().last().unwrap().instance_id, 0);
}
