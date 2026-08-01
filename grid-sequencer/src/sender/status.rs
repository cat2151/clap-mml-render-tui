use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum GridConnectionPhase {
    Idle,
    Connecting,
    WaitingForPatches,
    PatchSetting,
    Ready,
    Error(String),
}

impl GridConnectionPhase {
    pub fn accepts_notes(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridProgress {
    pub completed: usize,
    pub total: usize,
}

/// grid の1行（= CLAP instance 1つ）の準備段階。
///
/// 起動時の待ち時間は「サーバー側の instance 初期化」と「patch のロード」の
/// 2段階からなり、どちらも instance id = 行番号 の順に1つずつ進む。行の色を
/// この段階で塗り分けることで、待ち時間そのものを進捗表示にする。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridRowReadiness {
    /// instance がまだ構築されていない。
    Pending,
    /// サーバー側の instance 初期化だけ済み、patch は未ロード。
    InstanceReady,
    /// patch のロードまで済み、鳴らせる。
    Prepared,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridConnectionStatus {
    pub phase: GridConnectionPhase,
    pub last_send: Option<Duration>,
    pub limiter_reduction_db: f32,
    pub buffer_multiplier: u16,
    pub underrun_frames: u64,
    pub server_startup: Option<GridProgress>,
    pub patch_setting: Option<GridProgress>,
    /// 進行中の段が始まった時刻。確定値が出るまでの経過時間表示に使う。
    pub stage_started_at: Option<Instant>,
    /// サーバー起動（CLAP インスタンス生成）の確定所要時間。
    pub server_startup_elapsed: Option<Duration>,
    /// 音色ロード（16 instance 分の patch prepare）の確定所要時間。
    pub patch_setting_elapsed: Option<Duration>,
    /// 待機 bank への先読みロードの進捗。`phase` は動かさない（演奏中に走るため）。
    /// `completed` は成功も失敗も数える。ステータス行の表示にだけ使う。
    pub preload: GridProgress,
    /// 先読みロードで1件でも失敗したか。立っていたら bank を差し替えてはいけない。
    pub preload_failed: bool,
    /// 出力バッファを上限まで厚くしてもフレームドロップが止まらない状態が続いたか。
    /// 画面側はこれを見てシングルバッファリングへ落ちる（[`crate::single_buffer`]）。
    pub overloaded: bool,
}

impl Default for GridConnectionStatus {
    fn default() -> Self {
        Self {
            phase: GridConnectionPhase::Idle,
            last_send: None,
            limiter_reduction_db: 0.0,
            buffer_multiplier: super::adaptive_buffer::INITIAL_BUFFER_MULTIPLIER,
            underrun_frames: 0,
            server_startup: None,
            patch_setting: None,
            stage_started_at: None,
            server_startup_elapsed: None,
            patch_setting_elapsed: None,
            preload: GridProgress {
                completed: 0,
                total: 0,
            },
            preload_failed: false,
            overloaded: false,
        }
    }
}

impl GridConnectionStatus {
    pub fn label(&self) -> String {
        match &self.phase {
            GridConnectionPhase::Idle => "idle".to_string(),
            GridConnectionPhase::Connecting => self
                .server_startup
                .map(|progress| {
                    format!("starting server {}/{}", progress.completed, progress.total)
                })
                .unwrap_or_else(|| "starting server".to_string()),
            GridConnectionPhase::WaitingForPatches => "waiting for patches".to_string(),
            GridConnectionPhase::PatchSetting => self
                .patch_setting
                .map(|progress| format!("patches {}/{}", progress.completed, progress.total))
                .unwrap_or_else(|| "patches".to_string()),
            GridConnectionPhase::Ready => "ready".to_string(),
            GridConnectionPhase::Error(message) => message.clone(),
        }
    }

    /// 指定行（= instance id）の準備段階。grid のグレーアウト表示に使う。
    ///
    /// `Idle` を `Prepared` にしているのは、MIDI sender を持たないテストモードと
    /// 画面離脱後がこの phase であり、どちらも「準備中」ではないため。
    pub fn row_readiness(&self, row: usize) -> GridRowReadiness {
        match &self.phase {
            GridConnectionPhase::Idle | GridConnectionPhase::Ready => GridRowReadiness::Prepared,
            GridConnectionPhase::Connecting => {
                if row < self.server_startup.map_or(0, |progress| progress.completed) {
                    GridRowReadiness::InstanceReady
                } else {
                    GridRowReadiness::Pending
                }
            }
            GridConnectionPhase::WaitingForPatches => GridRowReadiness::InstanceReady,
            GridConnectionPhase::PatchSetting => {
                if row < self.patch_setting.map_or(0, |progress| progress.completed) {
                    GridRowReadiness::Prepared
                } else {
                    GridRowReadiness::InstanceReady
                }
            }
            GridConnectionPhase::Error(_) => GridRowReadiness::Pending,
        }
    }

    /// 起動待ちの最中か（中央 overlay を出すかどうかの判定）。
    pub fn is_preparing(&self) -> bool {
        matches!(
            self.phase,
            GridConnectionPhase::Connecting
                | GridConnectionPhase::WaitingForPatches
                | GridConnectionPhase::PatchSetting
        )
    }

    pub fn error_message(&self) -> Option<&str> {
        match &self.phase {
            GridConnectionPhase::Error(message) => Some(message),
            _ => None,
        }
    }

    /// 段1・段2それぞれの経過時間。確定していなければ、進行中の段だけ
    /// `stage_started_at` からの経過を返す（overlay で数字が動くようにするため）。
    pub fn stage_elapsed(&self, now: Instant) -> (Option<Duration>, Option<Duration>) {
        let running = self
            .stage_started_at
            .map(|started| now.saturating_duration_since(started));
        match &self.phase {
            GridConnectionPhase::Connecting => (self.server_startup_elapsed.or(running), None),
            GridConnectionPhase::WaitingForPatches | GridConnectionPhase::PatchSetting => (
                self.server_startup_elapsed,
                self.patch_setting_elapsed.or(running),
            ),
            _ => (self.server_startup_elapsed, self.patch_setting_elapsed),
        }
    }

    pub(super) fn begin_connecting(&mut self) {
        self.phase = GridConnectionPhase::Connecting;
        self.last_send = None;
        self.limiter_reduction_db = 0.0;
        self.buffer_multiplier = super::adaptive_buffer::INITIAL_BUFFER_MULTIPLIER;
        self.underrun_frames = 0;
        self.server_startup = None;
        self.patch_setting = None;
        self.stage_started_at = Some(Instant::now());
        self.server_startup_elapsed = None;
        self.patch_setting_elapsed = None;
    }

    pub(super) fn update_server_startup(&mut self, completed: usize, total: usize) {
        self.server_startup = Some(GridProgress { completed, total });
    }

    pub(super) fn wait_for_patches(&mut self, elapsed: Duration) {
        self.phase = GridConnectionPhase::WaitingForPatches;
        self.last_send = Some(elapsed);
        self.server_startup_elapsed = Some(elapsed);
        self.stage_started_at = Some(Instant::now());
    }

    /// `server_elapsed` は StartServer 経由で既に確定していれば採用しない。
    /// その経路の Prepare は起動済みサーバーへの再確認なので、ほぼ 0ms になり
    /// 本来のサーバー起動時間を上書きしてしまうため。
    pub(super) fn begin_patch_setting(&mut self, total: usize, server_elapsed: Duration) {
        self.phase = GridConnectionPhase::PatchSetting;
        self.patch_setting = Some(GridProgress {
            completed: 0,
            total,
        });
        self.server_startup_elapsed.get_or_insert(server_elapsed);
        self.patch_setting_elapsed = None;
        self.stage_started_at = Some(Instant::now());
    }

    pub(super) fn update_patch_setting(&mut self, completed: usize, total: usize) {
        self.patch_setting = Some(GridProgress { completed, total });
    }

    pub(super) fn finish_patch_setting(&mut self, elapsed: Duration) {
        self.patch_setting_elapsed = Some(elapsed);
    }

    pub(super) fn apply_result(
        &mut self,
        result: anyhow::Result<cmrt_realtime_play::LimiterMeter>,
        elapsed: Option<Duration>,
        idle_on_success: bool,
    ) {
        self.phase = match &result {
            Ok(_) if idle_on_success => GridConnectionPhase::Idle,
            Ok(_) => GridConnectionPhase::Ready,
            Err(error) => GridConnectionPhase::Error(error.to_string()),
        };
        if let Ok(meter) = result {
            self.update_limiter_meter(meter);
        }
        self.last_send = elapsed;
    }

    pub(super) fn update_limiter_meter(&mut self, meter: cmrt_realtime_play::LimiterMeter) {
        self.limiter_reduction_db = meter.peak_reduction_db.max(meter.current_reduction_db);
    }

    /// 先読みロードを1件投げた。UI スレッドから、送信の直前に呼ぶ。
    pub(super) fn begin_preload_step(&mut self) {
        self.preload.total = self.preload.total.saturating_add(1);
    }

    /// 先読みロードを1件終えた。送信スレッドから呼ぶ。
    pub(super) fn record_preload_step(&mut self, succeeded: bool) {
        self.preload.completed = self.preload.completed.saturating_add(1);
        if !succeeded {
            self.preload_failed = true;
        }
    }

    /// 先読みの集計を初期化する。新しいサイクルの先読みを始める前に呼ぶ。
    pub(super) fn reset_preload(&mut self) {
        self.preload = GridProgress {
            completed: 0,
            total: 0,
        };
        self.preload_failed = false;
    }

    /// 慢性的なフレームドロップが成立した。
    ///
    /// `begin_connecting()` では降ろさない。シングルバッファリングはサイクルごとに
    /// `prepare()`（= `begin_connecting()`）を通るので、そこで消すと latch にならない。
    pub(super) fn mark_overloaded(&mut self) {
        self.overloaded = true;
    }

    /// 判定を白紙へ戻す。画面を離れる（`Stop`）ときだけ呼ぶ。
    pub(super) fn clear_overload(&mut self) {
        self.overloaded = false;
    }

    pub(super) fn update_adaptive_buffer(&mut self, multiplier: u16, underrun_frames: u64) {
        self.buffer_multiplier = multiplier;
        self.underrun_frames = underrun_frames;
    }
}
