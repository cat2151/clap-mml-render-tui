//! MML オーバーレイ専用の MIDI 送信。
//!
//! オーバーレイを開くと現在の画面の演奏は止まる（呼び出し側が止める）ので、
//! 音源インスタンスは keyboard 画面と同じ 0 番を借りる。音色を指定しなければ
//! realtime server の既定音色（init saw）で鳴る。
//!
//! 送信は 2 系統ある。打鍵ごとの 1 音は offset なしの生 MIDI で即座に、行ぜんぶの
//! 演奏は live timeline へ絶対秒つきで積む（[`line_playback`] を参照）。
//!
//! 接続確立は初回に数百 ms 掛かることがあるため、送信はワーカースレッドへ逃がして
//! 入力の手応えを落とさない。

mod line_playback;

use std::{
    sync::{mpsc, Arc},
    thread::JoinHandle,
};

use cmrt_chord::TimedMidiEvent;
use cmrt_realtime_play::RealtimePlayServerSupervisor;

use line_playback::LinePlayback;

/// オーバーレイが借りる音源インスタンス。
pub(crate) const MML_OVERLAY_INSTANCE: u8 = 0;

enum SenderCommand {
    /// 音源をこの音色で使えるようにする。`None` なら既定音色。
    /// オーバーレイを開いた時点と、音色を選び直したときに走らせる。
    Prepare(Option<String>),
    Send(Vec<[u8; 3]>),
    /// 走っている行の演奏を止めてから、この行を頭から積む。
    PlayLine(Vec<TimedMidiEvent>),
    StopLine,
    Shutdown,
}

pub struct MmlOverlaySender {
    tx: mpsc::Sender<SenderCommand>,
    worker: Option<JoinHandle<()>>,
}

impl MmlOverlaySender {
    /// `sample_rate_hz` は live timeline を張るときにサーバーへ渡す。
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>, sample_rate_hz: f64) -> Self {
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("mml-overlay-midi-sender".to_string())
            .spawn(move || run_sender(rx, supervisor, sample_rate_hz))
            .expect("MML overlay MIDI sender thread should start");
        Self {
            tx,
            worker: Some(worker),
        }
    }

    pub fn prepare(&self, patch: Option<&str>) {
        let _ = self
            .tx
            .send(SenderCommand::Prepare(patch.map(str::to_string)));
    }

    pub fn send(&self, messages: Vec<[u8; 3]>) {
        if messages.is_empty() {
            return;
        }
        let _ = self.tx.send(SenderCommand::Send(messages));
    }

    /// 1 行ぶんのフレーズを、書かれた音長のまま演奏する。
    /// 空で呼ぶと、走っている演奏を止めるだけになる。
    pub fn play_line(&self, events: Vec<TimedMidiEvent>) {
        let _ = self.tx.send(SenderCommand::PlayLine(events));
    }

    pub fn stop_line(&self) {
        let _ = self.tx.send(SenderCommand::StopLine);
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

pub(crate) fn log_error(message: String) {
    let _ = cmrt_tui_core::logging::append_log_line_to_file(&format!("mml-overlay: {message}"));
}

fn run_sender(
    rx: mpsc::Receiver<SenderCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    sample_rate_hz: f64,
) {
    let mut line_playback = LinePlayback::new(sample_rate_hz);
    while let Ok(command) = rx.recv() {
        match command {
            SenderCommand::Prepare(patch) => {
                if let Err(error) =
                    supervisor.prepare_live_patch(MML_OVERLAY_INSTANCE, patch.as_deref())
                {
                    log_error(format!(
                        "action=mml-overlay-prepare event=error patch={patch:?} error=\"{error:#}\""
                    ));
                }
            }
            SenderCommand::Send(messages) => {
                // 行の演奏中に打鍵すると音が重なる。打鍵の 1 音を優先して先に止める。
                line_playback.stop(&supervisor);
                if let Err(error) = supervisor.send_midi(MML_OVERLAY_INSTANCE, &messages) {
                    log_error(format!(
                        "action=mml-overlay-send event=error error=\"{error:#}\""
                    ));
                }
            }
            SenderCommand::PlayLine(events) => line_playback.play(&supervisor, &events),
            SenderCommand::StopLine => line_playback.stop(&supervisor),
            SenderCommand::Shutdown => {
                line_playback.stop(&supervisor);
                break;
            }
        }
    }
}
