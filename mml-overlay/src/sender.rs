//! MML オーバーレイ専用の MIDI 送信。
//!
//! オーバーレイを開くと現在の画面の演奏は止まる（呼び出し側が止める）ので、
//! 音源インスタンスは keyboard 画面と同じ 0 番を借りる。音色を指定しなければ
//! realtime server の既定音色（init saw）で鳴る。
//!
//! 送信は 2 系統ある。打鍵ごとの 1 音は offset なしの生 MIDI で即座に、行ぜんぶの
//! 演奏は live timeline へ絶対秒つきで積む（[`line_playback`] を参照）。
//!
//! **どちらの系統も [`voice::Voice`] を通す。** 「鳴っているものを止める」はこの
//! ワーカースレッドだけが持つ（[`sounding::Sounding`] が唯一の記録）。上位の
//! [`crate::state`] は note off を組み立てないし、音源の状態も持たない。
//!
//! 接続確立は初回に数百 ms 掛かることがあるため、送信はワーカースレッドへ逃がして
//! 入力の手応えを落とさない。

mod line_playback;
mod sink;
mod sounding;
mod voice;

use std::{
    sync::{mpsc, Arc},
    thread::JoinHandle,
};

use cmrt_chord::TimedMidiEvent;
use cmrt_realtime_play::RealtimePlayServerSupervisor;

use voice::Voice;

/// オーバーレイが借りる音源インスタンス。
pub(crate) const MML_OVERLAY_INSTANCE: u8 = 0;

enum SenderCommand {
    /// 音源をこの音色で使えるようにする。`None` なら既定音色。
    /// オーバーレイを開いた時点と、音色を選び直したときに走らせる。
    Prepare(Option<String>),
    /// 鳴っているものを止めてから、この note on を送る。
    Send(Vec<[u8; 3]>),
    /// 鳴っているものを止めてから、この行を頭から積む。空なら止めるだけ。
    PlayLine(Vec<TimedMidiEvent>),
    /// 鳴っているものを止める。
    Stop,
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

    /// 打鍵の 1 音を鳴らす。渡すのは note on だけでよい。
    /// 前に鳴っていたものは受け取った側が止める。
    pub fn send(&self, messages: Vec<[u8; 3]>) {
        if messages.is_empty() {
            return;
        }
        let _ = self.tx.send(SenderCommand::Send(messages));
    }

    /// 1 行ぶんのフレーズを、書かれた音長のまま演奏する。
    /// 空で呼ぶと、鳴っているものを止めるだけになる。
    pub fn play_line(&self, events: Vec<TimedMidiEvent>) {
        let _ = self.tx.send(SenderCommand::PlayLine(events));
    }

    /// 鳴っているものを止める。打鍵の音か行の演奏かは呼び出し側が気にしなくてよい。
    pub fn stop(&self) {
        let _ = self.tx.send(SenderCommand::Stop);
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

pub(crate) use crate::log_line;

pub(crate) fn log_error(message: String) {
    log_line(message);
}

/// コマンドを [`Voice`] へ流すだけ。ここに条件分岐を足さないこと。
///
/// 「鳴らす前に止める」は [`Voice`] の中で閉じている。ここで「今は鳴っていないはず
/// だから止めなくてよい」と判断を挟むと、判断材料を 2 か所が持つ形へ逆戻りする。
fn run_sender(
    rx: mpsc::Receiver<SenderCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    sample_rate_hz: f64,
) {
    let mut voice = Voice::new(sample_rate_hz);
    let sink = &*supervisor;
    while let Ok(command) = rx.recv() {
        match command {
            SenderCommand::Prepare(patch) => voice.prepare(sink, patch.as_deref()),
            SenderCommand::Send(messages) => voice.play_notes(sink, &messages),
            SenderCommand::PlayLine(events) => voice.play_line(sink, &events),
            SenderCommand::Stop => voice.stop(sink, "stop"),
            SenderCommand::Shutdown => {
                voice.stop(sink, "shutdown");
                break;
            }
        }
    }
}
