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
//! 接続確立や patch load は数秒掛かることがあるため、送信はワーカースレッドへ逃がす。
//! note gate もこの worker が「note on の送信成功後」から数える。gate 待ちには
//! `recv_timeout` を使い、次の操作が来たら待ちを即座に打ち切って前の音を止める。
//!
//! 待ちの相手は 2 つある（[`voice::Wake`]）。gate の期限と、repeat の次の周を積む時刻の
//! 早いほうまで待ち、時間切れならその片方だけを片づけてまた待つ。**repeat の周回は
//! この worker のタイマーで積むが、積む中身は絶対秒なので鳴る位置は時計に左右されない。**

mod line_playback;
mod sink;
mod sounding;
mod voice;

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use crate::line_play::LineProgram;

use sink::SoundSink;
use voice::{Voice, Wake};

/// オーバーレイが借りる音源インスタンス。
pub(crate) const MML_OVERLAY_INSTANCE: u8 = 0;

enum SenderCommandKind {
    /// 音源をこの音色で使えるようにする。`None` なら既定音色。
    /// オーバーレイを開いた時点と、音色を選び直したときに走らせる。
    Prepare {
        patch: Option<String>,
    },
    /// 必要なら音色を読み込み、鳴っているものを止めてから note on を送る。
    PlayNotes {
        patch: Option<String>,
        messages: Vec<[u8; 3]>,
        gate: Duration,
    },
    /// 鳴っているものを止めてから、この行を頭から積む。空なら止めるだけ。
    PlayLine {
        patch: Option<String>,
        program: LineProgram,
    },
    /// 鳴っているものを止める。
    Stop,
    Shutdown,
}

impl SenderCommandKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Prepare { .. } => "prepare",
            Self::PlayNotes { .. } => "notes",
            Self::PlayLine { .. } => "line",
            Self::Stop => "stop",
            Self::Shutdown => "shutdown",
        }
    }
}

struct SenderCommand {
    id: u64,
    queued_at: Instant,
    kind: SenderCommandKind,
}

/// sender worker の現在状態。TUI は読み取りだけ行う。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MmlOverlaySenderStatus {
    pub(crate) command_id: u64,
    pub(crate) loading: bool,
    pub(crate) loading_patch: Option<String>,
    pub(crate) sounding: Vec<u8>,
}

impl MmlOverlaySenderStatus {
    pub fn command_id(&self) -> u64 {
        self.command_id
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn loading_patch(&self) -> Option<&str> {
        self.loading_patch.as_deref()
    }

    pub fn sounding(&self) -> &[u8] {
        &self.sounding
    }
}

pub struct MmlOverlaySender {
    tx: mpsc::Sender<SenderCommand>,
    next_command_id: AtomicU64,
    latest_command_id: Arc<AtomicU64>,
    status: Arc<Mutex<MmlOverlaySenderStatus>>,
    worker: Option<JoinHandle<()>>,
}

impl MmlOverlaySender {
    /// `sample_rate_hz` は live timeline を張るときにサーバーへ渡す。
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>, sample_rate_hz: f64) -> Self {
        let (tx, rx) = mpsc::channel();
        let latest_command_id = Arc::new(AtomicU64::new(0));
        let status = Arc::new(Mutex::new(MmlOverlaySenderStatus::default()));
        let worker_latest_command_id = Arc::clone(&latest_command_id);
        let worker_status = Arc::clone(&status);
        let worker = std::thread::Builder::new()
            .name("mml-overlay-midi-sender".to_string())
            .spawn(move || {
                run_sender(
                    rx,
                    supervisor,
                    sample_rate_hz,
                    worker_latest_command_id,
                    worker_status,
                )
            })
            .expect("MML overlay MIDI sender thread should start");
        Self {
            tx,
            next_command_id: AtomicU64::new(1),
            latest_command_id,
            status,
            worker: Some(worker),
        }
    }

    pub fn prepare(&self, patch: Option<&str>) -> u64 {
        self.enqueue(SenderCommandKind::Prepare {
            patch: patch.map(str::to_string),
        })
    }

    /// 打鍵の 1 音を鳴らす。渡すのは note on だけでよい。
    /// 前に鳴っていたものは受け取った側が止める。
    pub fn send(&self, patch: Option<&str>, messages: Vec<[u8; 3]>, gate: Duration) -> u64 {
        if messages.is_empty() {
            return self.stop();
        }
        self.enqueue(SenderCommandKind::PlayNotes {
            patch: patch.map(str::to_string),
            messages,
            gate,
        })
    }

    /// 1 行ぶんのフレーズを、書かれた音長のまま演奏する。
    /// 空で呼ぶと、鳴っているものを止めるだけになる。
    pub fn play_line(&self, patch: Option<&str>, program: LineProgram) -> u64 {
        self.enqueue(SenderCommandKind::PlayLine {
            patch: patch.map(str::to_string),
            program,
        })
    }

    /// 鳴っているものを止める。打鍵の音か行の演奏かは呼び出し側が気にしなくてよい。
    pub fn stop(&self) -> u64 {
        self.enqueue(SenderCommandKind::Stop)
    }

    pub fn status(&self) -> MmlOverlaySenderStatus {
        self.status.lock().unwrap().clone()
    }

    fn enqueue(&self, kind: SenderCommandKind) -> u64 {
        let id = self.next_command_id.fetch_add(1, Ordering::Relaxed);
        self.latest_command_id.store(id, Ordering::Release);
        if self
            .tx
            .send(SenderCommand {
                id,
                queued_at: Instant::now(),
                kind,
            })
            .is_err()
        {
            log_error(format!(
                "action=mml-overlay-command event=enqueue-error command_id={id}"
            ));
        }
        id
    }
}

impl Drop for MmlOverlaySender {
    fn drop(&mut self) {
        self.enqueue(SenderCommandKind::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(crate) use crate::log_line;

pub(crate) fn log_error(message: String) {
    log_line(message);
}

fn run_sender<S: SoundSink + Send + Sync + 'static>(
    rx: mpsc::Receiver<SenderCommand>,
    sink: Arc<S>,
    sample_rate_hz: f64,
    latest_command_id: Arc<AtomicU64>,
    status: Arc<Mutex<MmlOverlaySenderStatus>>,
) {
    let mut voice = Voice::new(sample_rate_hz);
    loop {
        let received = match voice.next_wake(Instant::now()) {
            Some((wake, wait)) => match rx.recv_timeout(wait) {
                Ok(command) => command,
                // 待ちが切れた。次の操作は来ていないので、起きた理由のほうを片づける。
                Err(RecvTimeoutError::Timeout) => {
                    match wake {
                        Wake::Gate => {
                            log_line(format!(
                                "action=mml-overlay-gate-expired command_id={}",
                                status.lock().unwrap().command_id
                            ));
                            voice.stop(&*sink, "gate");
                            status.lock().unwrap().sounding.clear();
                        }
                        // 継ぎ足しは止めない。ここで stop を通すと毎周継ぎ目が出る。
                        Wake::Repeat => voice.pump_repeat(&*sink, Instant::now()),
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(command) => command,
                Err(_) => break,
            },
        };
        let command = newest_queued_command(received, &rx);
        let name = command.kind.name();
        let queue_ms = command.queued_at.elapsed().as_millis();
        let started_at = Instant::now();
        log_line(format!(
            "action=mml-overlay-command event=start command_id={} command={name} queue_ms={queue_ms}",
            command.id
        ));
        voice.begin_command(command.id);
        let shutdown = matches!(&command.kind, SenderCommandKind::Shutdown);
        begin_status(&status, command.id);
        match command.kind {
            SenderCommandKind::Prepare { patch } => {
                prepare_if_needed(&mut voice, &*sink, &status, patch.as_deref());
            }
            SenderCommandKind::PlayNotes {
                patch,
                messages,
                gate,
            } => {
                let ready = prepare_if_needed(&mut voice, &*sink, &status, patch.as_deref());
                if ready && !is_superseded(command.id, &latest_command_id) {
                    if voice.play_notes(&*sink, &messages, gate) {
                        status.lock().unwrap().sounding = note_on_pitches(&messages);
                    }
                } else if ready {
                    log_superseded_after_load(command.id, &latest_command_id);
                }
            }
            SenderCommandKind::PlayLine { patch, program } => {
                let ready = prepare_if_needed(&mut voice, &*sink, &status, patch.as_deref());
                if ready && !is_superseded(command.id, &latest_command_id) {
                    voice.play_line(&*sink, &program);
                } else if ready {
                    log_superseded_after_load(command.id, &latest_command_id);
                }
            }
            SenderCommandKind::Stop => voice.stop(&*sink, "stop"),
            SenderCommandKind::Shutdown => {
                voice.stop(&*sink, "shutdown");
            }
        }
        log_line(format!(
            "action=mml-overlay-command event=finished command_id={} command={name} \
             elapsed_ms={}",
            command.id,
            started_at.elapsed().as_millis()
        ));
        if shutdown {
            break;
        }
    }
}

fn newest_queued_command(
    mut command: SenderCommand,
    rx: &mpsc::Receiver<SenderCommand>,
) -> SenderCommand {
    while let Ok(newer) = rx.try_recv() {
        log_line(format!(
            "action=mml-overlay-command event=superseded command_id={} by_command_id={}",
            command.id, newer.id
        ));
        command = newer;
    }
    command
}

fn begin_status(status: &Mutex<MmlOverlaySenderStatus>, command_id: u64) {
    *status.lock().unwrap() = MmlOverlaySenderStatus {
        command_id,
        ..MmlOverlaySenderStatus::default()
    };
}

fn prepare_if_needed(
    voice: &mut Voice,
    sink: &impl SoundSink,
    status: &Mutex<MmlOverlaySenderStatus>,
    patch: Option<&str>,
) -> bool {
    if voice.is_patch_ready(patch) {
        return true;
    }
    {
        let mut status = status.lock().unwrap();
        status.loading = true;
        status.loading_patch = patch.map(str::to_string);
        status.sounding.clear();
    }
    let ready = voice.prepare(sink, patch);
    let mut status = status.lock().unwrap();
    status.loading = false;
    status.loading_patch = None;
    ready
}

fn is_superseded(command_id: u64, latest_command_id: &AtomicU64) -> bool {
    latest_command_id.load(Ordering::Acquire) > command_id
}

fn log_superseded_after_load(command_id: u64, latest_command_id: &AtomicU64) {
    log_line(format!(
        "action=mml-overlay-command event=superseded-after-load command_id={command_id} \
         by_command_id={}",
        latest_command_id.load(Ordering::Acquire)
    ));
}

fn note_on_pitches(messages: &[[u8; 3]]) -> Vec<u8> {
    messages
        .iter()
        .filter(|message| message[0] == crate::NOTE_ON && message[2] > 0)
        .map(|message| message[1])
        .collect()
}

#[cfg(test)]
mod tests;
