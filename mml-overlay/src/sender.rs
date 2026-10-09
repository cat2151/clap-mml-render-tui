//! MML オーバーレイと、同じ即時試聴経路を借りるホスト機能の MIDI 送信。
//!
//! オーバーレイを開くときやホストが即時試聴するときは、現在の画面の演奏を呼び出し側で
//! 止めてから使う。音源インスタンスは keyboard 画面と同じ 0 番を借りる。音色を指定
//! しなければ realtime server の既定音色（init saw）で鳴る。
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
//! 待ちの相手は 3 つある（[`voice::Wake`]）。gate の期限、repeat の次の周を積む時刻、
//! 裏で読んでいる音色の完了確認のいちばん早いものまで待ち、時間切れならそれだけを
//! 片づけてまた待つ。**repeat の周回は
//! この worker のタイマーで積むが、積む中身は絶対秒なので鳴る位置は時計に左右されない。**

mod fade_out;
mod layers;
mod line_playback;
mod live_patch;
mod prepare;
mod queue;
#[cfg(any(test, feature = "test-support"))]
mod recording;
mod sink;
mod sounding;
mod sounding_lines;
mod status;
mod step_loop;
mod voice;
mod worker;

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use crate::line_play::LineProgram;

pub use layers::LineLayer;
pub use live_patch::LivePatch;
use queue::WorkerMessage;
#[cfg(any(test, feature = "test-support"))]
pub use recording::{RecordingSink, SinkOperation};
use sink::SoundSink;
use sounding_lines::SoundingLines;
pub use status::{MmlOverlayLinePlayback, MmlOverlayPreload, MmlOverlaySenderStatus};
use step_loop::StepLoopEdit;
pub use step_loop::{StepHit, StepLoop, StepShot};
use worker::run_sender;

/// オーバーレイが借りる音源インスタンス。
pub(crate) const MML_OVERLAY_INSTANCE: u8 = 0;

enum SenderCommandKind {
    /// 音源をこの音色で使えるようにする。音色が `None` なら既定音色。
    /// オーバーレイを開いた時点と、音色を選び直したときに走らせる。
    Prepare {
        patch: LivePatch,
    },
    /// 必要なら音色を読み込み、鳴っているものを止めてから note on を送る。
    PlayNotes {
        patch: LivePatch,
        messages: Vec<[u8; 3]>,
        gate: Duration,
    },
    /// 鳴っているものを止めてから、この行を頭から積む。空なら止めるだけ。
    PlayLine {
        patch: LivePatch,
        program: LineProgram,
        stop_before_prepare: bool,
    },
    /// 鳴っているものを止めてから、周期を固定した打点ループを頭から回す。
    PlayStepLoop {
        patch: LivePatch,
        step_loop: StepLoop,
    },
    /// 複数 instance の one-shot を 1 本の timeline として鳴らす。
    PlayLayers {
        layers: Vec<LineLayer>,
    },
    /// 鳴っているものを止める。
    Stop,
    /// 何もしない。列で待っている前の command（準備待ちの行など）を鳴らさずに終わらせる。
    /// 鳴っている音には触れない。
    Supersede,
    Shutdown,
}

impl SenderCommandKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Prepare { .. } => "prepare",
            Self::PlayNotes { .. } => "notes",
            Self::PlayLine { .. } => "line",
            Self::PlayStepLoop { .. } => "step-loop",
            Self::PlayLayers { .. } => "layers",
            Self::Stop => "stop",
            Self::Supersede => "supersede",
            Self::Shutdown => "shutdown",
        }
    }
}

struct SenderCommand {
    id: u64,
    queued_at: Instant,
    kind: SenderCommandKind,
}

pub struct MmlOverlaySender {
    tx: mpsc::Sender<WorkerMessage>,
    /// fadeout だけは worker を通さず、呼び出し側のスレッドからこれへ送る。
    fader: Arc<dyn SoundSink + Send + Sync>,
    sounding_lines: SoundingLines,
    next_command_id: AtomicU64,
    latest_command_id: Arc<AtomicU64>,
    /// `Drop` が立てる。worker は先読みの決着を待っている最中でもこれを見て抜ける。
    shutting_down: Arc<AtomicBool>,
    status: Arc<Mutex<MmlOverlaySenderStatus>>,
    worker: Option<JoinHandle<()>>,
}

impl MmlOverlaySender {
    /// `sample_rate_hz` は live timeline を張るときにサーバーへ渡す。
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>, sample_rate_hz: f64) -> Self {
        Self::spawn(supervisor, sample_rate_hz)
    }

    /// server の代わりに `sink` へ送る sender。他 crate のテストが、送った内容を数えるために使う。
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_recording_sink(sink: Arc<RecordingSink>, sample_rate_hz: f64) -> Self {
        Self::spawn(sink, sample_rate_hz)
    }

    fn spawn<S: SoundSink + Send + Sync + 'static>(sink: Arc<S>, sample_rate_hz: f64) -> Self {
        let (tx, rx) = mpsc::channel();
        let latest_command_id = Arc::new(AtomicU64::new(0));
        let status = Arc::new(Mutex::new(MmlOverlaySenderStatus::default()));
        let worker_latest_command_id = Arc::clone(&latest_command_id);
        let shutting_down = Arc::new(AtomicBool::new(false));
        let worker_shutting_down = Arc::clone(&shutting_down);
        let worker_status = Arc::clone(&status);
        let sounding_lines = SoundingLines::default();
        let worker_sounding_lines = sounding_lines.clone();
        let fader: Arc<dyn SoundSink + Send + Sync> = Arc::clone(&sink) as _;
        let worker = std::thread::Builder::new()
            .name("mml-overlay-midi-sender".to_string())
            .spawn(move || {
                run_sender(
                    rx,
                    sink,
                    sample_rate_hz,
                    worker_latest_command_id,
                    worker_shutting_down,
                    worker_status,
                    worker_sounding_lines,
                )
            })
            .expect("MML overlay MIDI sender thread should start");
        Self {
            tx,
            fader,
            sounding_lines,
            next_command_id: AtomicU64::new(1),
            latest_command_id,
            shutting_down,
            status,
            worker: Some(worker),
        }
    }

    pub fn prepare(&self, patch: impl Into<LivePatch>) -> u64 {
        self.enqueue(SenderCommandKind::Prepare {
            patch: patch.into(),
        })
    }

    /// 打鍵の 1 音を鳴らす。渡すのは note on だけでよい。
    /// 前に鳴っていたものは受け取った側が止める。
    pub fn send(&self, patch: impl Into<LivePatch>, messages: Vec<[u8; 3]>, gate: Duration) -> u64 {
        if messages.is_empty() {
            return self.stop();
        }
        self.enqueue(SenderCommandKind::PlayNotes {
            patch: patch.into(),
            messages,
            gate,
        })
    }

    /// 1 行ぶんのフレーズを、書かれた音長のまま演奏する。
    /// 空で呼ぶと、鳴っているものを止めるだけになる。
    pub fn play_line(&self, patch: impl Into<LivePatch>, program: LineProgram) -> u64 {
        self.enqueue(SenderCommandKind::PlayLine {
            patch: patch.into(),
            program,
            stop_before_prepare: false,
        })
    }

    /// 古い予約イベントを消してから音色を準備し、単発の行を送る。
    /// Stop と PlayLine を別々に積むと queue の圧縮で Stop が消えるため、1 command にする。
    pub fn stop_and_play_line(&self, patch: impl Into<LivePatch>, program: LineProgram) -> u64 {
        if program.is_silent() {
            return self.stop();
        }
        self.enqueue(SenderCommandKind::PlayLine {
            patch: patch.into(),
            program,
            stop_before_prepare: true,
        })
    }

    /// 鳴っているものを止め、周期を固定した打点ループを頭から回し始める。
    ///
    /// 打点が空でも止めずに時計を進める。回している間の打点は
    /// [`Self::update_step_loop`] で差し替え、まだ積んでいない時刻から効く。
    pub fn play_step_loop(&self, patch: impl Into<LivePatch>, step_loop: StepLoop) -> u64 {
        self.enqueue(SenderCommandKind::PlayStepLoop {
            patch: patch.into(),
            step_loop,
        })
    }

    /// 走っている打点ループの打点を差し替える。ループが無ければ何もしない。
    ///
    /// command ではないので、列で待っている操作を置き換えず、状態表示も変えない。
    /// 差し替えより後に積まれた command があれば、そちらの内容が勝つ。
    pub fn update_step_loop(&self, hits: Vec<StepHit>) {
        self.edit_step_loop(StepLoopEdit::Hits(hits));
    }

    /// 走っている打点ループに、周回とは別の 1 打をいま鳴らす。ループが無ければ何もしない。
    /// 同じ note の打点をすでに積んである時刻とは重ねない（[`StepShot`] の規則）。
    pub fn shoot_step_loop(&self, shot: StepShot) {
        self.edit_step_loop(StepLoopEdit::Shot(shot));
    }

    /// 走っている打点ループの [`StepLoop::horizon_seconds`] を替える。周回位置は変えない。
    pub fn set_step_loop_horizon(&self, seconds: f64) {
        self.edit_step_loop(StepLoopEdit::Horizon(seconds));
    }

    fn edit_step_loop(&self, edit: StepLoopEdit) {
        if self.tx.send(WorkerMessage::StepLoop(edit)).is_err() {
            log_error("action=mml-overlay-step-loop event=enqueue-error".to_string());
        }
    }

    /// 複数 instance の one-shot performance を、1 command / 1 timeline で鳴らす。
    /// 空で呼ぶと、鳴っているものを止めるだけになる。
    pub fn play_layers(&self, layers: Vec<LineLayer>) -> u64 {
        self.enqueue(SenderCommandKind::PlayLayers { layers })
    }

    /// 鳴っていない bank へ `patch` を裏で読んでおく。以後この音色の行は読み込み無しで鳴る。
    ///
    /// 列に積んだ前の操作を捨てず、読み込みの間も「読み込み中」を出さず、次の操作を
    /// 塞がない。状態は [`MmlOverlaySenderStatus::preload`] で見る。
    pub fn preload(&self, patch: impl Into<LivePatch>) {
        if self.tx.send(WorkerMessage::Preload(patch.into())).is_err() {
            log_error("action=mml-overlay-preload event=enqueue-error".to_string());
        }
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
            .send(WorkerMessage::Command(SenderCommand {
                id,
                queued_at: Instant::now(),
                kind,
            }))
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
        self.shutting_down.store(true, Ordering::Release);
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

#[cfg(test)]
mod tests;
