//! grid sequencer から realtime play server への MIDI 送信ハンドル。
//!
//! ここは UI スレッドから触る窓口（コマンドの送信と status の読み取り）だけを持ち、
//! サーバーとのやり取りは [`worker`] のスレッドが受け持つ。

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

use cmrt_realtime_play::{RealtimePlayServerSupervisor, TimelineMidiEvent};

mod adaptive_buffer;
mod gain_summary;
mod overload;
mod status;
mod worker;

static NEXT_ROW_PATCH_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_LIVE_TIMELINE_ID: AtomicU64 = AtomicU64::new(1);

pub use status::{
    GridConnectionPhase, GridConnectionStatus, GridProgress, GridRowPatchPhase, GridRowPatchStatus,
    GridRowReadiness,
};

enum GridMidiCommand {
    StartServer,
    Send {
        events: Vec<TimelineMidiEvent>,
        queued_at: Instant,
        pump_lateness: std::time::Duration,
    },
    BeginTimeline {
        config: cmrt_realtime_play::LiveTimelineConfig,
    },
    /// 鳴っている bank の音色を丸ごと差し替える。`stop_live_all()` を伴うので
    /// ロード中は無音になる。起動時と `r` キー用。
    Prepare {
        patches: Vec<(u8, Option<String>)>,
    },
    /// 待機 bank への先読みロード1件。**演奏中に走る**ので `stop_live_all()` を
    /// 呼ばず、phase も動かさない（`accepts_notes()` を落とすと演奏が止まる）。
    Preload {
        instance_id: u8,
        patch: Option<String>,
    },
    /// 演奏中の bank にある1行だけを差し替える。ほかの行は止めない。
    SetRowPatch {
        request_id: u64,
        queued_at: Instant,
        reason: &'static str,
        row: usize,
        instance_id: u8,
        patch: Option<String>,
    },
    /// 先読みロードの終了。厚くしていた出力バッファを戻す。
    PreloadFinished,
    SetGains {
        gains_db: Vec<f32>,
    },
    SetAutoGain {
        enabled: bool,
    },
    Stop,
    Shutdown,
}

pub struct GridMidiSender {
    tx: mpsc::Sender<GridMidiCommand>,
    status: Arc<Mutex<GridConnectionStatus>>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    worker: Option<JoinHandle<()>>,
}

impl GridMidiSender {
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(GridConnectionStatus::default()));
        let worker_status = Arc::clone(&status);
        let worker_supervisor = Arc::clone(&supervisor);
        let worker = std::thread::Builder::new()
            .name("grid-sequencer-midi-sender".to_string())
            .spawn(move || worker::run_midi_sender(rx, worker_supervisor, worker_status))
            .expect("grid sequencer MIDI sender thread should start");
        Self {
            tx,
            status,
            supervisor,
            worker: Some(worker),
        }
    }

    pub fn start_server(&self) {
        self.status.lock().unwrap().begin_connecting();
        let _ = self.tx.send(GridMidiCommand::StartServer);
    }

    pub fn send_scheduled(
        &self,
        events: Vec<TimelineMidiEvent>,
        pump_lateness: std::time::Duration,
    ) {
        if !events.is_empty() {
            let _ = self.tx.send(GridMidiCommand::Send {
                events,
                queued_at: Instant::now(),
                pump_lateness,
            });
        }
    }

    pub fn begin_timeline(&self, sample_rate_hz: f64) -> u64 {
        let timeline_id = NEXT_LIVE_TIMELINE_ID.fetch_add(1, Ordering::Relaxed).max(1);
        let _ = self.tx.send(GridMidiCommand::BeginTimeline {
            config: cmrt_realtime_play::LiveTimelineConfig {
                timeline_id,
                sample_rate_hz,
                tempo_bpm: crate::BPM as f64,
                time_signature_numerator: 4,
                time_signature_denominator: 4,
            },
        });
        timeline_id
    }

    pub fn prepare<'a>(&self, patches: impl Iterator<Item = (u8, Option<&'a str>)>) {
        self.status.lock().unwrap().begin_connecting();
        let patches = patches
            .map(|(instance_id, patch)| (instance_id, patch.map(str::to_string)))
            .collect::<Vec<_>>();
        let _ = self.tx.send(GridMidiCommand::Prepare { patches });
    }

    /// 新しいサイクルの先読みを始める。進捗と失敗フラグを初期化する。
    pub fn begin_preload_cycle(&self) {
        self.status.lock().unwrap().reset_preload();
    }

    /// 待機 bank へ patch を1件だけ先読みする。演奏は止めない。
    ///
    /// 1ステップにつき1件ずつ呼ぶこと。まとめて投げるとサーバーのレンダースレッドが
    /// 連続で止まり、出力リングを食い潰して underrun になる。
    pub fn preload(&self, instance_id: u8, patch: Option<&str>) {
        self.status.lock().unwrap().begin_preload_step();
        let _ = self.tx.send(GridMidiCommand::Preload {
            instance_id,
            patch: patch.map(str::to_string),
        });
    }

    /// 現在 bank の1行だけを差し替える。全 instance 用の `prepare` と違い、
    /// `stop_live_all()` は呼ばない。
    pub fn set_row_patch(
        &self,
        row: usize,
        instance_id: u8,
        patch: Option<&str>,
        reason: &'static str,
    ) -> u64 {
        let request_id = NEXT_ROW_PATCH_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        self.status.lock().unwrap().begin_row_patch_setting(row);
        if self
            .tx
            .send(GridMidiCommand::SetRowPatch {
                request_id,
                queued_at: Instant::now(),
                reason,
                row,
                instance_id,
                patch: patch.map(str::to_string),
            })
            .is_err()
        {
            self.status
                .lock()
                .unwrap()
                .finish_row_patch_setting(row, Some("MIDI worker stopped".to_string()));
        }
        request_id
    }

    /// 先読みを終える（全件送り終えた、または打ち切った）。バッファ厚を戻す。
    pub fn finish_preload(&self) {
        let _ = self.tx.send(GridMidiCommand::PreloadFinished);
    }

    /// instance ごとの音量差を dB で設定する（0.0 が等倍）。
    ///
    /// ゲインはサーバー側が保持するので、音色ロードで live を作り直しても残る。
    /// 変わったときだけ送ればよい。
    pub fn set_gains(&self, gains_db: Vec<f32>) {
        let _ = self.tx.send(GridMidiCommand::SetGains { gains_db });
    }

    pub fn set_auto_gain_enabled(&self, enabled: bool) {
        let _ = self.tx.send(GridMidiCommand::SetAutoGain { enabled });
    }

    pub fn stop(&self) {
        // 判定はここで降ろす。ワーカー側でも戻すが、キューを捌く前に画面へ入り直すと
        // 古い判定を読んでシングルバッファリングのまま再開してしまう。
        let mut status = self.status.lock().unwrap();
        status.clear_overload();
        status.clear_row_patch_setting();
        status.reset_timing_session();
        drop(status);
        let _ = self.tx.send(GridMidiCommand::Stop);
    }

    pub fn status(&self) -> GridConnectionStatus {
        let mut status = self.status.lock().unwrap().clone();
        if matches!(status.phase, GridConnectionPhase::Connecting) {
            if let Some(progress) = self.supervisor.startup_progress() {
                status.update_server_startup(
                    progress.initialized_instances,
                    progress.total_instances,
                );
            }
        }
        status
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

#[cfg(test)]
mod tests;
