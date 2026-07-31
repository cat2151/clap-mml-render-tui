use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use cmrt_realtime_play::{FastMidiEvent, LimiterMeter, RealtimePlayServerSupervisor};

mod adaptive_buffer;
mod status;

use adaptive_buffer::{AdaptiveBuffer, INITIAL_BUFFER_MULTIPLIER, RESTORE_BUFFER_MULTIPLIER};
pub use status::{GridConnectionPhase, GridConnectionStatus, GridProgress, GridRowReadiness};

const METER_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// 先読みロード中だけ厚くする出力バッファ。
///
/// patch のロードはサーバーのレンダースレッド上で同期実行され、その間リングが
/// 補充されない。実測で 1 patch あたり平均 27ms（最悪 150ms）かかるのに対し、
/// リングの余裕は 512 フレーム × multiplier ÷ 48kHz なので、既定の
/// `INITIAL_BUFFER_MULTIPLIER`（= 2、21ms）では足りない。16 なら 170ms 稼げて、
/// `LOOKAHEAD`（2ステップ = 230ms）にも収まる。
const PRELOAD_BUFFER_MULTIPLIER: u8 = 16;

enum GridMidiCommand {
    StartServer,
    Send {
        events: Vec<FastMidiEvent>,
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
    /// 先読みロードの終了。厚くしていた出力バッファを戻す。
    PreloadFinished,
    SetGains {
        gains_db: Vec<f32>,
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
            .spawn(move || run_midi_sender(rx, worker_supervisor, worker_status))
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

    pub fn send_scheduled(&self, events: Vec<FastMidiEvent>) {
        if !events.is_empty() {
            let _ = self.tx.send(GridMidiCommand::Send { events });
        }
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

    pub fn stop(&self) {
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

fn run_midi_sender(
    rx: mpsc::Receiver<GridMidiCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<GridConnectionStatus>>,
) {
    let mut adaptive_buffer = None;
    // 先読みロード中は出力バッファを厚くしている。戻し忘れを防ぐために持つ。
    let mut preloading = false;
    loop {
        let command = match rx.recv_timeout(METER_POLL_INTERVAL) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                poll_runtime_status(
                    supervisor.as_ref(),
                    &status,
                    &mut adaptive_buffer,
                    Instant::now(),
                );
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match command {
            GridMidiCommand::StartServer => {
                adaptive_buffer = None;
                let started = Instant::now();
                let result = supervisor.ensure_started_for_fast_midi();
                let server_elapsed = started.elapsed();
                log_startup_summary("start-server", server_elapsed, None, 0, result.is_ok());
                match result {
                    Ok(()) => status.lock().unwrap().wait_for_patches(server_elapsed),
                    Err(error) => apply(&status, Err(error), Some(server_elapsed), false),
                }
            }
            GridMidiCommand::Send { events } => {
                let started = Instant::now();
                let result = supervisor.send_live_events(&events);
                apply(&status, result, Some(started.elapsed()), false);
            }
            GridMidiCommand::Prepare { patches } => {
                adaptive_buffer = None;
                preloading = false;
                // サーバー起動（CLAP インスタンス生成）と音色ロードは所要時間の桁が
                // 違うので、別々に計測してどちらが支配的かを切り分けられるようにする。
                let server_started = Instant::now();
                let ensure = supervisor.ensure_started_for_fast_midi();
                let server_elapsed = server_started.elapsed();
                let patch_started = Instant::now();
                let result = ensure.and_then(|()| {
                    status
                        .lock()
                        .unwrap()
                        .begin_patch_setting(patches.len(), server_elapsed);
                    prepare_instances(supervisor.as_ref(), &patches, |completed, total| {
                        status
                            .lock()
                            .unwrap()
                            .update_patch_setting(completed, total);
                    })
                });
                let patch_elapsed = patch_started.elapsed();
                log_startup_summary(
                    "prepare",
                    server_elapsed,
                    Some(patch_elapsed),
                    patches.len(),
                    result.is_ok(),
                );
                status.lock().unwrap().finish_patch_setting(patch_elapsed);
                if result.is_ok() {
                    let now = Instant::now();
                    let buffer = AdaptiveBuffer::new(now, supervisor.underrun_frames());
                    status
                        .lock()
                        .unwrap()
                        .update_adaptive_buffer(buffer.multiplier(), buffer.underrun_frames());
                    adaptive_buffer = Some(buffer);
                }
                apply(
                    &status,
                    result.map(|()| supervisor.limiter_meter()),
                    Some(server_elapsed + patch_elapsed),
                    false,
                );
            }
            GridMidiCommand::Preload { instance_id, patch } => {
                // 初回だけ出力バッファを厚くする。ロード中はレンダースレッドが
                // 止まるので、その間ぶんを先に貯めておかないと underrun になる。
                if !preloading {
                    preloading = true;
                    let _ =
                        supervisor.set_connected_live_buffer_multiplier(PRELOAD_BUFFER_MULTIPLIER);
                }
                let started = Instant::now();
                let result = supervisor.prepare_live_patch(instance_id, patch.as_deref());
                match &result {
                    Ok(()) => crate::log_line(&format!(
                        "grid-sequencer: preload instance={instance_id} ms={}",
                        started.elapsed().as_millis()
                    )),
                    Err(error) => crate::log_line(&format!(
                        "grid-sequencer: preload failed instance={instance_id} error=\"{error:#}\""
                    )),
                }
                status.lock().unwrap().record_preload_step(result.is_ok());
            }
            GridMidiCommand::PreloadFinished => {
                if preloading {
                    preloading = false;
                    // 適応バッファが持っている厚さへ戻す。まだ無ければ既定へ。
                    let multiplier = adaptive_buffer
                        .map(AdaptiveBuffer::multiplier)
                        .unwrap_or(INITIAL_BUFFER_MULTIPLIER);
                    let _ = supervisor.set_connected_live_buffer_multiplier(multiplier);
                }
            }
            GridMidiCommand::SetGains { gains_db } => {
                let mut failed = None;
                for (instance_id, gain_db) in gains_db.iter().enumerate() {
                    // 古いサーバーはこのコマンドを知らない。音量差が付かないだけなので
                    // ログにとどめ、再生は続ける。
                    if let Err(error) =
                        supervisor.set_live_instance_gain_db(instance_id as u8, *gain_db)
                    {
                        failed = Some(format!("{error:#}"));
                        break;
                    }
                }
                crate::log_line(&format!(
                    "grid-sequencer: gains instances={} boosted={} result={}",
                    gains_db.len(),
                    describe_boosted(&gains_db),
                    match &failed {
                        Some(error) => format!("error \"{error}\""),
                        None => "ok".to_string(),
                    },
                ));
            }
            GridMidiCommand::Stop => {
                adaptive_buffer = None;
                preloading = false;
                let started = Instant::now();
                let result = supervisor
                    .stop_live_all()
                    .and_then(|()| {
                        supervisor.set_connected_live_buffer_multiplier(RESTORE_BUFFER_MULTIPLIER)
                    })
                    .map(|()| LimiterMeter::default());
                status
                    .lock()
                    .unwrap()
                    .update_adaptive_buffer(RESTORE_BUFFER_MULTIPLIER, 0);
                apply(&status, result, Some(started.elapsed()), true);
            }
            GridMidiCommand::Shutdown => break,
        }
    }
}

/// 起動待ちの内訳を1行にまとめてログへ残す。
///
/// 「サーバー起動が支配的か、音色ロードが支配的か」を後から log.txt だけで
/// 切り分けられるようにするためのもの。サーバー側の内訳は同じログファイルの
/// `cmrt-server-timing:` 行と突き合わせる。
fn log_startup_summary(
    stage: &str,
    server: Duration,
    patch: Option<Duration>,
    instances: usize,
    succeeded: bool,
) {
    let patch_ms = patch.map_or_else(|| "-".to_string(), |patch| patch.as_millis().to_string());
    let total_ms = (server + patch.unwrap_or_default()).as_millis();
    crate::log_line(&format!(
        "grid-sequencer: startup stage={stage} server_ms={} patch_ms={patch_ms} \
         total_ms={total_ms} instances={instances} result={}",
        server.as_millis(),
        if succeeded { "ok" } else { "error" }
    ));
}

/// 指定された instance の音色をまとめて差し替える。`stop_live_all()` を伴うので
/// この間は無音になる。鳴っている bank ぶんだけを渡すこと（待機 bank は
/// [`GridMidiCommand::Preload`] が演奏中に裏で仕込む）。
fn prepare_instances(
    supervisor: &RealtimePlayServerSupervisor,
    patches: &[(u8, Option<String>)],
    mut report_progress: impl FnMut(usize, usize),
) -> anyhow::Result<()> {
    let available = supervisor.live_instance_count();
    if let Some((instance_id, _)) = patches
        .iter()
        .find(|(instance_id, _)| usize::from(*instance_id) >= available)
    {
        anyhow::bail!("instance {instance_id} is outside the server range 0..{available}");
    }
    supervisor.stop_live_all()?;
    supervisor.set_live_buffer_multiplier(INITIAL_BUFFER_MULTIPLIER)?;
    report_progress(0, patches.len());
    for (completed, (instance_id, patch)) in patches.iter().enumerate() {
        if let Err(error) = supervisor
            .prepare_live_patch(*instance_id, patch.as_deref())
            .with_context(|| {
                format!(
                    "grid instance {instance_id} patch prepare failed (patch={:?})",
                    patch.as_deref()
                )
            })
        {
            let _ = supervisor.stop_live_all();
            return Err(error);
        }
        report_progress(completed + 1, patches.len());
    }
    Ok(())
}

fn poll_runtime_status(
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<GridConnectionStatus>,
    adaptive_buffer: &mut Option<AdaptiveBuffer>,
    now: Instant,
) {
    status
        .lock()
        .unwrap()
        .update_limiter_meter(supervisor.limiter_meter());
    if !status.lock().unwrap().phase.accepts_notes() {
        return;
    }
    let Some(buffer) = adaptive_buffer.as_mut() else {
        return;
    };
    let adjustment = buffer.observe(now, supervisor.underrun_frames());
    if let Some(multiplier) = adjustment {
        let previous = status.lock().unwrap().buffer_multiplier;
        if let Err(error) = supervisor.set_live_buffer_multiplier(multiplier) {
            apply(status, Err(error), None, false);
            return;
        }
        let reason = if multiplier > previous {
            "underrun"
        } else {
            "stable"
        };
        crate::log_line(&format!(
            "grid-sequencer: buffer auto {previous} -> {multiplier} reason={reason}"
        ));
    }
    status
        .lock()
        .unwrap()
        .update_adaptive_buffer(buffer.multiplier(), buffer.underrun_frames());
}

fn apply(
    status: &Mutex<GridConnectionStatus>,
    result: anyhow::Result<LimiterMeter>,
    elapsed: Option<std::time::Duration>,
    idle_on_success: bool,
) {
    if let Err(error) = &result {
        crate::log_line(&format!("grid-sequencer: MIDI worker error: {error:#}"));
    }
    status
        .lock()
        .unwrap()
        .apply_result(result, elapsed, idle_on_success);
}

/// 0 dB でない行だけを `row2:+6dB` のように並べる。全部 0 dB なら `none`。
fn describe_boosted(gains_db: &[f32]) -> String {
    let boosted = gains_db
        .iter()
        .enumerate()
        .filter(|(_, gain)| **gain != 0.0)
        .map(|(index, gain)| format!("row{}:{gain:+}dB", index + 1))
        .collect::<Vec<_>>();
    if boosted.is_empty() {
        "none".to_string()
    } else {
        boosted.join(",")
    }
}

#[cfg(test)]
mod tests;
