//! MML オーバーレイ専用の MIDI 送信。
//!
//! オーバーレイを開くと現在の画面の演奏は止まる（呼び出し側が止める）ので、
//! 音源インスタンスは keyboard 画面と同じ 0 番を借りる。patch は指定せず、
//! realtime server の既定音色（init saw）で鳴らす。
//!
//! 接続確立は初回に数百 ms 掛かることがあるため、送信はワーカースレッドへ逃がして
//! 入力の手応えを落とさない。

use std::{
    sync::{mpsc, Arc},
    thread::JoinHandle,
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

/// オーバーレイが借りる音源インスタンス。
const MML_OVERLAY_INSTANCE: u8 = 0;

enum SenderCommand {
    /// 音源を既定音色で使えるようにする。オーバーレイを開いた時点で先に走らせる。
    Prepare,
    Send(Vec<[u8; 3]>),
    Shutdown,
}

pub struct MmlOverlaySender {
    tx: mpsc::Sender<SenderCommand>,
    worker: Option<JoinHandle<()>>,
}

impl MmlOverlaySender {
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>) -> Self {
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("mml-overlay-midi-sender".to_string())
            .spawn(move || run_sender(rx, supervisor))
            .expect("MML overlay MIDI sender thread should start");
        Self {
            tx,
            worker: Some(worker),
        }
    }

    pub fn prepare(&self) {
        let _ = self.tx.send(SenderCommand::Prepare);
    }

    pub fn send(&self, messages: Vec<[u8; 3]>) {
        if messages.is_empty() {
            return;
        }
        let _ = self.tx.send(SenderCommand::Send(messages));
    }
}

impl Drop for MmlOverlaySender {
    fn drop(&mut self) {
        let _ = self.tx.send(SenderCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn log_error(message: String) {
    let _ = cmrt_tui_core::logging::append_log_line_to_file(&format!("mml-overlay: {message}"));
}

fn run_sender(rx: mpsc::Receiver<SenderCommand>, supervisor: Arc<RealtimePlayServerSupervisor>) {
    while let Ok(command) = rx.recv() {
        match command {
            SenderCommand::Prepare => {
                if let Err(error) = supervisor.prepare_live_patch(MML_OVERLAY_INSTANCE, None) {
                    log_error(format!(
                        "action=mml-overlay-prepare event=error error=\"{error:#}\""
                    ));
                }
            }
            SenderCommand::Send(messages) => {
                if let Err(error) = supervisor.send_midi(MML_OVERLAY_INSTANCE, &messages) {
                    log_error(format!(
                        "action=mml-overlay-send event=error error=\"{error:#}\""
                    ));
                }
            }
            SenderCommand::Shutdown => break,
        }
    }
}
