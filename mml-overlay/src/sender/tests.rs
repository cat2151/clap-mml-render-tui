use std::{
    sync::{
        atomic::{AtomicU64, AtomicUsize},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use cmrt_realtime_play::{LiveTimelineConfig, TimelineMidiEvent};

use super::*;
use crate::line_play::{LinePerformance, LineProgram};
use crate::{NOTE_OFF, NOTE_ON};

#[derive(Clone, Debug)]
struct RecordedMidi {
    at: Instant,
    messages: Vec<[u8; 3]>,
}

#[derive(Default)]
struct FakeSink {
    prepare_delay: Duration,
    /// 準備を失敗させる理由。`None` なら成功する。
    prepare_error: Option<String>,
    prepared: Mutex<Vec<Option<String>>>,
    midi: Mutex<Vec<RecordedMidi>>,
    /// timeline を張った回数。repeat が張り直していないことを worker 越しに見る。
    begins: AtomicUsize,
    /// timeline を含む realtime 音源を hard stop した回数。
    stops: AtomicUsize,
    /// timeline へ積んだ秒。継ぎ足しが伸び続けることを見る。
    timeline_seconds: Mutex<Vec<f64>>,
}

impl FakeSink {
    fn begins(&self) -> usize {
        self.begins.load(Ordering::Acquire)
    }

    fn timeline_seconds(&self) -> Vec<f64> {
        self.timeline_seconds.lock().unwrap().clone()
    }

    fn stops(&self) -> usize {
        self.stops.load(Ordering::Acquire)
    }
}

impl SoundSink for FakeSink {
    fn prepare_patch(&self, patch: Option<&str>) -> sink::SinkResult {
        std::thread::sleep(self.prepare_delay);
        self.prepared
            .lock()
            .unwrap()
            .push(patch.map(str::to_string));
        match &self.prepare_error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn send_midi(&self, messages: &[[u8; 3]]) -> sink::SinkResult {
        self.midi.lock().unwrap().push(RecordedMidi {
            at: Instant::now(),
            messages: messages.to_vec(),
        });
        Ok(())
    }

    fn stop_all(&self) -> sink::SinkResult {
        self.stops.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn begin_timeline(&self, _config: LiveTimelineConfig) -> sink::SinkResult {
        self.begins.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn send_timeline_events(&self, events: &[TimelineMidiEvent]) -> sink::SinkResult {
        self.timeline_seconds
            .lock()
            .unwrap()
            .extend(events.iter().map(|event| event.timeline_seconds));
        Ok(())
    }
}

struct Harness {
    tx: mpsc::Sender<SenderCommand>,
    latest: Arc<AtomicU64>,
    status: Arc<Mutex<MmlOverlaySenderStatus>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Harness {
    fn spawn(sink: Arc<FakeSink>) -> Self {
        let (tx, rx) = mpsc::channel();
        let latest = Arc::new(AtomicU64::new(0));
        let status = Arc::new(Mutex::new(MmlOverlaySenderStatus::default()));
        let worker_latest = Arc::clone(&latest);
        let worker_status = Arc::clone(&status);
        let worker = std::thread::spawn(move || {
            run_sender(rx, sink, 48_000.0, worker_latest, worker_status);
        });
        Self {
            tx,
            latest,
            status,
            worker: Some(worker),
        }
    }

    fn send(&self, id: u64, kind: SenderCommandKind) {
        self.latest.store(id, Ordering::Release);
        self.tx
            .send(SenderCommand {
                id,
                queued_at: Instant::now(),
                kind,
            })
            .unwrap();
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let id = self.latest.load(Ordering::Acquire) + 1;
        self.send(id, SenderCommandKind::Shutdown);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

fn notes(patch: &str, pitch: u8, gate: Duration) -> SenderCommandKind {
    SenderCommandKind::PlayNotes {
        patch: Some(patch.to_string()),
        messages: vec![[NOTE_ON, pitch, 127]],
        gate,
    }
}

/// 1 周 `loop_seconds` の行を鳴らす指示。`repeat` が worker の待ちに効く。
fn line(loop_seconds: f64, repeat: bool) -> SenderCommandKind {
    SenderCommandKind::PlayLine {
        patch: Some("ready.sfz".to_string()),
        program: LineProgram {
            performance: LinePerformance {
                events: vec![cmrt_chord::TimedMidiEvent {
                    seconds: 0.0,
                    message: [NOTE_ON, 60, 127],
                }],
                loop_seconds,
            },
            repeat,
            filters: Default::default(),
        },
    }
}

fn wait_until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline, "condition timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn event_time(sink: &FakeSink, status: u8, pitch: u8) -> Option<Instant> {
    sink.midi
        .lock()
        .unwrap()
        .iter()
        .find(|record| {
            record
                .messages
                .iter()
                .any(|message| message[0] == status && message[1] == pitch)
        })
        .map(|record| record.at)
}

#[test]
fn slow_patch_load_is_shown_and_does_not_consume_the_note_gate() {
    let sink = Arc::new(FakeSink {
        prepare_delay: Duration::from_millis(80),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, notes("slow.sfz", 60, Duration::from_millis(100)));

    wait_until(|| harness.status.lock().unwrap().is_loading());
    assert_eq!(
        harness.status.lock().unwrap().loading_patch(),
        Some("slow.sfz")
    );
    wait_until(|| event_time(&sink, NOTE_OFF, 60).is_some());

    let note_on = event_time(&sink, NOTE_ON, 60).unwrap();
    let note_off = event_time(&sink, NOTE_OFF, 60).unwrap();
    assert!(
        note_off.duration_since(note_on) >= Duration::from_millis(90),
        "note window was {:?}",
        note_off.duration_since(note_on)
    );
}

#[test]
fn next_note_interrupts_a_long_gate_without_waiting_for_it() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, notes("ready.sfz", 60, Duration::from_secs(2)));
    wait_until(|| event_time(&sink, NOTE_ON, 60).is_some());

    let next_requested = Instant::now();
    harness.send(2, notes("ready.sfz", 62, Duration::from_millis(100)));
    wait_until(|| event_time(&sink, NOTE_OFF, 60).is_some());

    assert!(event_time(&sink, NOTE_OFF, 60).unwrap() - next_requested < Duration::from_millis(200));
    wait_until(|| event_time(&sink, NOTE_ON, 62).is_some());
}

#[test]
fn a_new_request_during_load_suppresses_the_stale_preview() {
    let sink = Arc::new(FakeSink {
        prepare_delay: Duration::from_millis(80),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, notes("old.sfz", 60, Duration::from_millis(100)));
    wait_until(|| harness.status.lock().unwrap().is_loading());

    harness.send(2, notes("new.sfz", 64, Duration::from_millis(100)));
    wait_until(|| event_time(&sink, NOTE_ON, 64).is_some());

    assert_eq!(event_time(&sink, NOTE_ON, 60), None);
    assert_eq!(
        *sink.prepared.lock().unwrap(),
        vec![Some("old.sfz".to_string()), Some("new.sfz".to_string())]
    );
}

/// **worker が自分でループを回すこと。** `Voice` 側の継ぎ足しが正しくても、
/// `run_sender` の待ちが gate だけを見ていたら最初の先読みぶんで音が尽きる。
/// ここは実際に worker thread を回して、時間が経つと勝手に伸びることを見る。
#[test]
fn the_worker_tops_up_a_repeating_line_on_its_own() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));

    // 1 周 60ms。先読み 4 秒ぶんを積んだ後、20ms ほどで次の周が要る。
    harness.send(1, line(0.06, true));
    wait_until(|| !sink.timeline_seconds().is_empty());
    let first = sink.timeline_seconds().len();
    wait_until(|| sink.timeline_seconds().len() > first);

    // 張り直していない。張り直すと毎周リセットが入って継ぎ目が出る。
    assert_eq!(sink.begins(), 1);
    let seconds = sink.timeline_seconds();
    assert!(
        seconds.windows(2).all(|pair| pair[1] > pair[0]),
        "timeline_seconds must keep going forward: {seconds:?}"
    );
    // 先読み 4 秒ぶんが最初の 1 回で積まれている。
    assert!(*seconds.last().unwrap() >= 4.0, "seconds={seconds:?}");
}

/// 止めたら worker も回すのをやめる。ここが残ると、誰も鳴らしていないつもりのまま
/// サーバーへ積み続ける。
#[test]
fn stopping_ends_the_worker_side_repeat() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(0.06, true));
    wait_until(|| !sink.timeline_seconds().is_empty());

    harness.send(2, SenderCommandKind::Stop);
    // 回っていれば 60ms ごとに伸びる。その 5 倍待てば Stop は確実に処理されている。
    std::thread::sleep(Duration::from_millis(300));
    let after_stop = sink.timeline_seconds().len();
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(sink.timeline_seconds().len(), after_stop);
}

/// repeat OFF は今までどおり 1 回積んで終わり。worker は寝たまま。
#[test]
fn without_repeat_the_worker_stays_asleep() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));

    harness.send(1, line(0.06, false));
    wait_until(|| !sink.timeline_seconds().is_empty());
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(sink.timeline_seconds().len(), 1);
    assert_eq!(sink.begins(), 1);
}

#[test]
fn preparing_an_already_ready_patch_stops_the_previous_line() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(1.0, false));
    wait_until(|| sink.begins() == 1);

    harness.send(
        2,
        SenderCommandKind::Prepare {
            patch: Some("ready.sfz".to_string()),
        },
    );
    wait_until(|| sink.stops() == 1);
}

/// **行の演奏では `status.sounding` が埋まらない**（打鍵の生 MIDI 専用の欄）。
///
/// 2026-09-09 実測。行を借りて鳴らすホスト機能（chord chart の preview など）が
/// 「いま鳴っているか」を [`MmlOverlaySenderStatus::sounding`] で判定できるか、を
/// **実サーバー無しで**確かめるための固定。sink は全部成功する FakeSink なので、
/// ここで埋まらないなら実サーバーでも埋まらない（成功側が上限）。
///
/// 打鍵（`PlayNotes`）を対照に置く。対照が無いと「まだ処理されていないから空」と
/// 区別が付かない。
#[test]
fn a_line_performance_leaves_the_sounding_status_empty() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));

    // 対照: 打鍵の 1 音なら埋まる。
    harness.send(1, notes("ready.sfz", 60, Duration::from_secs(5)));
    wait_until(|| !harness.status.lock().unwrap().sounding().is_empty());
    assert_eq!(harness.status.lock().unwrap().sounding(), [60]);

    harness.send(2, line(4.0, false));
    wait_until(|| sink.begins() == 1);

    assert!(
        harness.status.lock().unwrap().sounding().is_empty(),
        "行の演奏は sounding を埋めない: {:?}",
        harness.status.lock().unwrap().sounding()
    );
}

/// **空の行を積む指示は、鳴っている timeline を止める**（`Stop` と同じ経路）。
///
/// 行を借りて鳴らすホスト機能が「止める」を `play_line(空)` で表現してよいことの固定。
/// 2026-09-09 実測（`stop_all` が 1 回飛ぶ）。
#[test]
fn an_empty_line_stops_the_running_timeline() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(4.0, false));
    wait_until(|| sink.begins() == 1);
    assert_eq!(sink.stops(), 0, "まだ止めていないこと（対照）");

    harness.send(
        2,
        SenderCommandKind::PlayLine {
            patch: Some("ready.sfz".to_string()),
            program: LineProgram::silent(),
        },
    );

    wait_until(|| sink.stops() == 1);
    assert_eq!(sink.begins(), 1, "止めるだけで timeline は張り直さないこと");
}

/// 準備に失敗したら、**`loading` は必ず下ろし、理由は status に残す。**
///
/// 画面はこの 2 つで「音が鳴るまで」の overlay を閉じ、消えた理由を出す
/// （`app/src/tui/sound_startup_overlay.rs`）。理由を持ち帰れないと、
/// overlay が黙って消えて音も鳴らない状態になる。
#[test]
fn a_failed_preparation_lowers_loading_and_keeps_its_reason() {
    let sink = Arc::new(FakeSink {
        prepare_error: Some("play server が起動できません".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));

    harness.send(
        1,
        SenderCommandKind::Prepare {
            patch: Some("lead.fxp".to_string()),
        },
    );
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());

    let status = harness.status.lock().unwrap().clone();
    assert!(
        !status.is_loading(),
        "失敗しても loading は下ろすこと（overlay が出っぱなしになる）"
    );
    assert_eq!(status.prepare_error(), Some("play server が起動できません"));
}

/// 成功したら理由は消える。古い失敗が居座ると、鳴っているのに理由が出続ける。
#[test]
fn a_successful_preparation_clears_the_previous_reason() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.status.lock().unwrap().prepare_error = Some("stale".to_string());

    harness.send(1, SenderCommandKind::Prepare { patch: None });
    wait_until(|| !sink.prepared.lock().unwrap().is_empty());
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_none());
}

/// 失敗の直後に別の command が来ても、理由は消えない。
///
/// 失敗すると画面はまず overlay を閉じ、そのあと理由を読む。読む前に
/// `Stop` の 1 つでも挟まると理由が消える作りだと、「黙って消えて音も鳴らない」に戻る。
#[test]
fn the_reason_survives_the_next_command() {
    let sink = Arc::new(FakeSink {
        prepare_error: Some("boom".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, SenderCommandKind::Prepare { patch: None });
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());

    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| harness.status.lock().unwrap().command_id() == 2);

    assert_eq!(harness.status.lock().unwrap().prepare_error(), Some("boom"));
}
