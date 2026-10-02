use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use cmrt_realtime_play::{
    PatchVoicing, RealtimePlayServerSupervisor, TimelineId, TimelineMidiEvent,
};

mod connection;
mod patch_prepare;
mod status;
mod timeline;

use connection::{set_prepare_result, set_result, set_result_keeping_phase};
use patch_prepare::{run_plan, LivePatchState};
pub use status::{KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardVoicingStatus};
pub(crate) use timeline::next_timeline_id;
use timeline::{timeline_config, TimelineWorker};

pub(crate) const KEYBOARD_INSTANCE: u8 = 0;
/// コマンドが来ないときに timing 統計を見に行く間隔。
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug)]
pub(super) struct PatchRequest<'a> {
    pub(super) patch: Option<&'a str>,
    pub(super) known_voicing: Option<PatchVoicing>,
}

enum KeyboardMidiCommand {
    Send {
        messages: Vec<[u8; 3]>,
    },
    /// 新しい id で timeline を張る。予約済みの音は捨てられ、鳴っている音は離される。
    BeginTimeline(TimelineId),
    /// 張った timeline 上の絶対秒で予約する。
    SendTimeline(Vec<TimelineMidiEvent>),
    Stop,
    Prepare {
        buffer_multiplier: u8,
        patch: Option<String>,
        known_voicing: Option<PatchVoicing>,
        effect_chain: String,
    },
    SetBufferMultiplier(u8),
    SetPatch {
        note_offs: Vec<[u8; 3]>,
        patch: Option<String>,
        known_voicing: Option<PatchVoicing>,
    },
    /// 音色はそのままで、鳴っている音へ掛ける chain だけを差し替える。
    SetEffectChain(String),
    Shutdown,
}

pub struct KeyboardMidiSender {
    tx: mpsc::Sender<KeyboardMidiCommand>,
    status: Arc<Mutex<KeyboardConnectionStatus>>,
    /// 起動進捗を読むためだけの参照。**worker と同じ 1 本**（`Arc` 共有）で、
    /// ここから送信することはない。worker は patch load の同期待ちで数秒止まるので、
    /// 進捗をコマンド経由で聞くと、まさに知りたい時間帯だけ返事が来ない。
    supervisor: Arc<RealtimePlayServerSupervisor>,
    worker: Option<JoinHandle<()>>,
}

/// worker が起動時に受け取る設定。
struct WorkerConfig {
    buffer_multiplier: u8,
    /// play server の `sample_rate`（config.toml）。timeline を張るのに要る。
    sample_rate_hz: f64,
}

impl KeyboardMidiSender {
    /// `sample_rate_hz` は play server が使う config.toml の `sample_rate` と一致させること。
    pub fn new(
        supervisor: Arc<RealtimePlayServerSupervisor>,
        buffer_multiplier: u8,
        sample_rate_hz: f64,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(KeyboardConnectionStatus::new(buffer_multiplier)));
        let worker_status = Arc::clone(&status);
        let worker_supervisor = Arc::clone(&supervisor);
        let config = WorkerConfig {
            buffer_multiplier,
            sample_rate_hz,
        };
        let worker = std::thread::Builder::new()
            .name("keyboard-midi-sender".to_string())
            .spawn(move || run_midi_sender(rx, worker_supervisor, worker_status, config))
            .expect("keyboard MIDI sender thread should start");
        Self {
            tx,
            status,
            supervisor,
            worker: Some(worker),
        }
    }

    pub fn send(&self, messages: Vec<[u8; 3]>, _patch: Option<&str>) {
        let _ = self.tx.send(KeyboardMidiCommand::Send { messages });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(KeyboardMidiCommand::Stop);
    }

    pub(crate) fn begin_timeline(&self, timeline_id: TimelineId) {
        let _ = self
            .tx
            .send(KeyboardMidiCommand::BeginTimeline(timeline_id));
    }

    pub(crate) fn send_timeline(&self, events: Vec<TimelineMidiEvent>) {
        let _ = self.tx.send(KeyboardMidiCommand::SendTimeline(events));
    }

    pub fn prepare(
        &self,
        buffer_multiplier: u8,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
        effect_chain: &str,
    ) {
        self.status
            .lock()
            .unwrap()
            .begin_connecting(buffer_multiplier, patch, known_voicing);
        let _ = self.tx.send(KeyboardMidiCommand::Prepare {
            buffer_multiplier,
            patch: patch.map(str::to_string),
            known_voicing,
            effect_chain: effect_chain.to_string(),
        });
    }

    /// 鳴っている音へ掛ける chain を差し替える（`chain_json` の綴り。空で外す）。
    /// 以後の音色の差し替えにも、この chain が載り続ける。
    pub fn set_effect_chain(&self, chain_json: &str) {
        let _ = self
            .tx
            .send(KeyboardMidiCommand::SetEffectChain(chain_json.to_string()));
    }

    pub fn set_buffer_multiplier(&self, multiplier: u8) {
        let _ = self
            .tx
            .send(KeyboardMidiCommand::SetBufferMultiplier(multiplier));
    }

    pub fn set_patch(
        &self,
        note_offs: Vec<[u8; 3]>,
        _previous_patch: Option<&str>,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        self.status
            .lock()
            .unwrap()
            .begin_patch_setting(patch, known_voicing);
        let _ = self.tx.send(KeyboardMidiCommand::SetPatch {
            note_offs,
            patch: patch.map(str::to_string),
            known_voicing,
        });
    }

    pub fn status(&self) -> KeyboardConnectionStatus {
        let mut status = self.status.lock().unwrap().clone();
        // 「何本目の instance か」は supervisor だけが知っている。二重に数えず、
        // 起動待ちの最中だけここで詰め直す。
        if status.phase == KeyboardConnectionPhase::Connecting {
            status.server_startup = self
                .supervisor
                .startup_progress()
                .map(|progress| (progress.initialized_instances, progress.total_instances));
        }
        status
    }
}

impl Drop for KeyboardMidiSender {
    fn drop(&mut self) {
        let _ = self.tx.send(KeyboardMidiCommand::Stop);
        let _ = self.tx.send(KeyboardMidiCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_midi_sender(
    rx: mpsc::Receiver<KeyboardMidiCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<KeyboardConnectionStatus>>,
    config: WorkerConfig,
) {
    let mut buffer_multiplier = config.buffer_multiplier;
    let mut live_patch = LivePatchState::default();
    let mut timeline = TimelineWorker::default();
    let _ = supervisor.remember_live_buffer_multiplier(u16::from(buffer_multiplier));
    loop {
        timeline.poll(&supervisor, Instant::now());
        let command = match rx.recv_timeout(IDLE_POLL_INTERVAL) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match command {
            KeyboardMidiCommand::Send { messages } => {
                let started = Instant::now();
                let result = supervisor
                    .send_midi(KEYBOARD_INSTANCE, &messages)
                    .map(|_| ());
                set_result(
                    &status,
                    buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    false,
                );
            }
            KeyboardMidiCommand::BeginTimeline(timeline_id) => {
                let started = Instant::now();
                let result = timeline.begin(
                    &supervisor,
                    timeline_config(timeline_id, config.sample_rate_hz),
                );
                set_result_keeping_phase(&status, result, started.elapsed());
            }
            KeyboardMidiCommand::SendTimeline(events) => {
                let started = Instant::now();
                let result = timeline.send(&supervisor, &events, started);
                set_result_keeping_phase(&status, result, started.elapsed());
            }
            KeyboardMidiCommand::Stop => {
                timeline.stop();
                let started = Instant::now();
                let result = supervisor.stop_live_instance(KEYBOARD_INSTANCE);
                set_result(
                    &status,
                    buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    true,
                );
            }
            KeyboardMidiCommand::Prepare {
                buffer_multiplier: requested_multiplier,
                patch,
                known_voicing,
                effect_chain,
            } => {
                buffer_multiplier = requested_multiplier;
                let plan = live_patch.plan_patch(patch.clone(), known_voicing, Some(effect_chain));
                let started = Instant::now();
                // **play server の起動待ちはここ。** 抜けるまで phase は
                // `Connecting` のまま（＝ overlay は「play server 起動」を出す）。
                // 以前はこの行の前に `PatchSetting` へ移していたので、実際には
                // サーバーの起動（実測 1.7〜6.5 秒）を待っているあいだ
                // 「patch setting...」と出ていた。
                let server_started = supervisor.ensure_started_for_fast_midi();
                status.lock().unwrap().phase = KeyboardConnectionPhase::PatchSetting;
                let result = server_started
                    .and_then(|()| {
                        supervisor.set_live_buffer_multiplier(u16::from(buffer_multiplier))
                    })
                    .and_then(|()| run_plan(supervisor.as_ref(), &plan));
                live_patch.finish_patch(result.is_ok());
                set_prepare_result(
                    &status,
                    buffer_multiplier,
                    result,
                    PatchRequest {
                        patch: patch.as_deref(),
                        known_voicing,
                    },
                    Some(started.elapsed()),
                );
            }
            KeyboardMidiCommand::SetBufferMultiplier(multiplier) => {
                buffer_multiplier = multiplier;
                let started = Instant::now();
                let result = supervisor.set_live_buffer_multiplier(u16::from(multiplier));
                set_result(
                    &status,
                    buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    false,
                );
            }
            KeyboardMidiCommand::SetPatch {
                note_offs,
                patch,
                known_voicing,
            } => {
                let started = Instant::now();
                let note_off_result = if note_offs.is_empty() {
                    Ok(())
                } else {
                    supervisor
                        .send_midi(KEYBOARD_INSTANCE, &note_offs)
                        .map(|_| ())
                };
                let request = PatchRequest {
                    patch: patch.as_deref(),
                    known_voicing,
                };
                let plan = live_patch.plan_patch(patch.clone(), known_voicing, None);
                let result = note_off_result.and_then(|()| run_plan(supervisor.as_ref(), &plan));
                live_patch.finish_patch(result.is_ok());
                set_prepare_result(
                    &status,
                    buffer_multiplier,
                    result,
                    request,
                    Some(started.elapsed()),
                );
            }
            KeyboardMidiCommand::SetEffectChain(effect_chain) => {
                let plan = live_patch.plan_effect_chain(effect_chain);
                if plan.steps.is_empty() {
                    continue;
                }
                let started = Instant::now();
                let result = run_plan(supervisor.as_ref(), &plan).map(|_| ());
                set_result_keeping_phase(&status, result, started.elapsed());
            }
            KeyboardMidiCommand::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests;
