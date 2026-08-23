use std::{
    sync::{atomic::AtomicU64, mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_realtime_play::{LiveTimelineConfig, TimelineMidiEvent};

use super::*;
use crate::{NOTE_OFF, NOTE_ON};

#[derive(Clone, Debug)]
struct RecordedMidi {
    at: Instant,
    messages: Vec<[u8; 3]>,
}

#[derive(Default)]
struct FakeSink {
    prepare_delay: Duration,
    prepared: Mutex<Vec<Option<String>>>,
    midi: Mutex<Vec<RecordedMidi>>,
}

impl SoundSink for FakeSink {
    fn prepare_patch(&self, patch: Option<&str>) -> sink::SinkResult {
        std::thread::sleep(self.prepare_delay);
        self.prepared
            .lock()
            .unwrap()
            .push(patch.map(str::to_string));
        Ok(())
    }

    fn send_midi(&self, messages: &[[u8; 3]]) -> sink::SinkResult {
        self.midi.lock().unwrap().push(RecordedMidi {
            at: Instant::now(),
            messages: messages.to_vec(),
        });
        Ok(())
    }

    fn stop_all(&self) -> sink::SinkResult {
        Ok(())
    }

    fn begin_timeline(&self, _config: LiveTimelineConfig) -> sink::SinkResult {
        Ok(())
    }

    fn send_timeline_events(&self, _events: &[TimelineMidiEvent]) -> sink::SinkResult {
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
