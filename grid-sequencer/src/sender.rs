use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;
use cmrt_tui_core::keyboard_session_state::KeyboardTransport;

mod status;
mod worker;

pub use status::{GridConnectionPhase, GridConnectionStatus};
use worker::WorkerState;

/// live 再生のバッファ倍率。grid sequencer では UI から変更できないので固定。
const BUFFER_MULTIPLIER: u8 = 4;

enum GridMidiCommand {
    Send {
        /// `(offset_frames, message)`。offset はサーバーの現在の live 位置からのフレーム数。
        events: Vec<(u32, [u8; 3])>,
        patch: Option<String>,
    },
    /// 接続を張り直し、音色を適用する（画面へ入るとき）。
    Prepare {
        patch: Option<String>,
    },
    /// 鳴っている音を止めてから音色を差し替える（`r` で行0の patch が変わったとき）。
    SetPatch {
        note_offs: Vec<[u8; 3]>,
        previous_patch: Option<String>,
        patch: Option<String>,
    },
    Stop,
    Shutdown,
}

/// realtime play server への MIDI 送信を専用スレッドへ逃がす送信口。
///
/// UI スレッドはコマンドを投げるだけで、結果は `status()` のスナップショットで読む。
pub struct GridMidiSender {
    tx: mpsc::Sender<GridMidiCommand>,
    status: Arc<Mutex<GridConnectionStatus>>,
    worker: Option<JoinHandle<()>>,
}

impl GridMidiSender {
    pub fn new(
        supervisor: Arc<RealtimePlayServerSupervisor>,
        transport: KeyboardTransport,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(GridConnectionStatus::new(transport)));
        let worker_status = Arc::clone(&status);
        let worker = std::thread::Builder::new()
            .name("grid-sequencer-midi-sender".to_string())
            .spawn(move || run_midi_sender(rx, supervisor, worker_status, transport))
            .expect("grid sequencer MIDI sender thread should start");
        Self {
            tx,
            status,
            worker: Some(worker),
        }
    }

    /// 全メッセージを即座（offset 0）に鳴らす。
    pub fn send(&self, messages: Vec<[u8; 3]>, patch: Option<&str>) {
        let events = messages.into_iter().map(|message| (0, message)).collect();
        self.send_scheduled(events, patch);
    }

    /// `(offset_frames, message)` の並びで送る。offset はサーバーの現在の live 位置から
    /// のフレーム数で、サーバー側でサンプル精度のスケジュールに載る。
    pub fn send_scheduled(&self, events: Vec<(u32, [u8; 3])>, patch: Option<&str>) {
        if events.is_empty() {
            return;
        }
        let _ = self.tx.send(GridMidiCommand::Send {
            events,
            patch: patch.map(str::to_string),
        });
    }

    pub fn prepare(&self, patch: Option<&str>) {
        self.status.lock().unwrap().begin_connecting(patch);
        let _ = self.tx.send(GridMidiCommand::Prepare {
            patch: patch.map(str::to_string),
        });
    }

    /// 音色を差し替える。live 中の `/midi` は patch を無視するサーバー仕様のため、
    /// note off は旧音色で送り切ってから `/live-patch` を通す。
    ///
    /// `/live-patch` はサーバー側の live session を作り直す（先読みで送信済みの
    /// イベントはそこで破棄される）ため、note off は offset 0 で構わない。
    pub fn set_patch(
        &self,
        note_offs: Vec<[u8; 3]>,
        previous_patch: Option<&str>,
        patch: Option<&str>,
    ) {
        self.status.lock().unwrap().begin_patch_setting(patch);
        let _ = self.tx.send(GridMidiCommand::SetPatch {
            note_offs,
            previous_patch: previous_patch.map(str::to_string),
            patch: patch.map(str::to_string),
        });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(GridMidiCommand::Stop);
    }

    pub fn status(&self) -> GridConnectionStatus {
        self.status.lock().unwrap().clone()
    }
}

impl Drop for GridMidiSender {
    fn drop(&mut self) {
        let _ = self.tx.send(GridMidiCommand::Stop);
        let _ = self.tx.send(GridMidiCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_midi_sender(
    rx: mpsc::Receiver<GridMidiCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<GridConnectionStatus>>,
    transport: KeyboardTransport,
) {
    let mut worker = WorkerState::new(transport);
    let _ = supervisor.remember_live_buffer_multiplier(BUFFER_MULTIPLIER);
    while let Ok(command) = rx.recv() {
        match command {
            GridMidiCommand::Send { events, patch } => {
                let started = Instant::now();
                let result = worker.send_midi(supervisor.as_ref(), &events, patch.as_deref());
                apply(&status, result, Some(started.elapsed()), false);
            }
            GridMidiCommand::Prepare { patch } => {
                let started = Instant::now();
                let _ = worker.stop(supervisor.as_ref());
                worker.disconnect();
                let result = worker.prepare_patch(supervisor.as_ref(), patch.as_deref());
                apply(&status, result, Some(started.elapsed()), false);
            }
            GridMidiCommand::SetPatch {
                note_offs,
                previous_patch,
                patch,
            } => {
                let started = Instant::now();
                // 旧音色のまま note off を送り切ってから差し替える。
                let note_off_result = if note_offs.is_empty() {
                    Ok(())
                } else {
                    let events = note_offs
                        .into_iter()
                        .map(|message| (0, message))
                        .collect::<Vec<_>>();
                    worker.send_midi(supervisor.as_ref(), &events, previous_patch.as_deref())
                };
                let patch_result = worker.prepare_patch(supervisor.as_ref(), patch.as_deref());
                apply(
                    &status,
                    note_off_result.and(patch_result),
                    Some(started.elapsed()),
                    false,
                );
            }
            GridMidiCommand::Stop => {
                let started = Instant::now();
                let result = worker.stop(supervisor.as_ref());
                apply(&status, result, Some(started.elapsed()), true);
            }
            GridMidiCommand::Shutdown => break,
        }
    }
}

fn apply(
    status: &Mutex<GridConnectionStatus>,
    result: anyhow::Result<()>,
    elapsed: Option<std::time::Duration>,
    idle_on_success: bool,
) {
    status
        .lock()
        .unwrap()
        .apply_result(result, elapsed, idle_on_success);
}

#[cfg(test)]
mod tests;
