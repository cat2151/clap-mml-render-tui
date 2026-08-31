//! 完了まで待ってよいコマンドの実装。
//!
//! ここに置いてよいのは「待っている間どのみち音が出ない」か「待ちが数十 ms で
//! 終わる」ものだけ。起動・全 instance のロード（`r` キー）・行差し替え・停止が
//! それにあたる。**待機 bank への先読みはここに無い**。演奏中に走るので待っては
//! いけず、受付と完了ポーリングは [`super::super::preload`] の状態機械が持つ。

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use cmrt_realtime_play::{LimiterMeter, RealtimePlayServerSupervisor};

use super::{
    super::super::{
        adaptive_buffer::{AdaptiveBuffer, INITIAL_BUFFER_MULTIPLIER, RESTORE_BUFFER_MULTIPLIER},
        gain_summary::describe_adjusted,
        overload::OverloadDetector,
        GridMidiCommand,
    },
    apply, SupervisorBackend,
};

/// 演奏中の**行差し替え**（[`GridMidiCommand::SetRowPatch`]）のあいだだけ確保する
/// 出力バッファの厚さ（下限）。
///
/// この経路のロードはサーバーの coordinator が同期で待つため、その間はどの bank も
/// render されずリングが補充されない（手動の行差し替えを無停止化しないのは
/// 意図した契約。`PLAN-grid-bank-worker-separation.md` の「前提と守る契約」4）。
/// 実測で 1 patch あたり平均 27ms（最悪 150ms）かかるのに対し、リングの余裕は
/// 512 フレーム × multiplier ÷ 48kHz なので、既定の `INITIAL_BUFFER_MULTIPLIER`
/// （= 2、21ms）では足りない。16 なら 170ms 稼げる。
///
/// 適応バッファが既にこれより厚ければ、そちらを保つ（薄くすると underrun を招く）。
///
/// **待機 bank への先読みでは使わない。** サーバーの bank worker 分離以降、
/// 先読み中も演奏 bank の render は止まらない。
const ROW_PATCH_BUFFER_MULTIPLIER: u16 = 16;

impl SupervisorBackend {
    /// 待ってよいコマンドを振り分ける。ホットパスの3種はここへ来ない。
    pub(super) fn dispatch_slow_command(&mut self, command: GridMidiCommand) {
        match command {
            GridMidiCommand::StartServer => self.start_server(),
            GridMidiCommand::BeginTimeline { config } => self.begin_timeline(config),
            GridMidiCommand::SetLiveTempo { change } => self.set_live_tempo(change),
            GridMidiCommand::Prepare { patches } => self.prepare(patches),
            GridMidiCommand::SetRowPatch {
                request_id,
                queued_at,
                reason,
                row,
                instance_id,
                patch,
            } => self.set_row_patch(request_id, queued_at, reason, row, instance_id, patch),
            GridMidiCommand::SetGains { gains } => self.set_gains(gains),
            GridMidiCommand::SetAutoGain { enabled } => self.set_auto_gain(enabled),
            GridMidiCommand::Stop => self.stop(),
            // ループが自分で捌く。ここへ来るのは分岐の付け忘れ。
            GridMidiCommand::Send { .. }
            | GridMidiCommand::Preload { .. }
            | GridMidiCommand::Shutdown => {
                debug_assert!(false, "hot path commands are handled by the command loop");
            }
        }
    }

    fn start_server(&mut self) {
        self.adaptive_buffer = None;
        let started = Instant::now();
        let result = self.supervisor.ensure_started_for_fast_midi();
        let server_elapsed = started.elapsed();
        log_startup_summary("start-server", server_elapsed, None, 0, result.is_ok());
        match result {
            Ok(()) => self.status.lock().unwrap().wait_for_patches(server_elapsed),
            Err(error) => apply(&self.status, Err(error), Some(server_elapsed), false),
        }
    }

    fn begin_timeline(&mut self, config: cmrt_realtime_play::LiveTimelineConfig) {
        let started = Instant::now();
        let result = self
            .supervisor
            .begin_live_timeline(config)
            .map(|()| self.supervisor.limiter_meter());
        crate::log_line(&format!(
            "grid-sequencer: timeline begin id={} sample_rate={} bpm={} result={}",
            config.timeline_id,
            config.sample_rate_hz,
            config.tempo_bpm,
            if result.is_ok() { "ok" } else { "error" },
        ));
        if result.is_ok() {
            self.timing_late_baseline = self.supervisor.timing_metrics().late_events_total;
            self.status
                .lock()
                .unwrap()
                .update_timing(cmrt_realtime_play::TimingMetrics::default());
        }
        apply(&self.status, result, Some(started.elapsed()), false);
    }

    fn set_live_tempo(&mut self, change: cmrt_realtime_play::LiveTempoChange) {
        let result = self.supervisor.set_live_tempo(change);
        // 失敗しても phase は動かさない。accepts_notes() を落とすと無音になる。
        // テンポが追従しないだけで、演奏そのものは続けられる。
        crate::log_line(&format!(
            "grid-sequencer: tempo-map id={} at_seconds={:.6} bpm={} result={}",
            change.timeline_id,
            change.at_seconds,
            change.tempo_bpm,
            match &result {
                Ok(()) => "ok".to_string(),
                Err(error) => format!("error \"{error:#}\""),
            },
        ));
    }

    fn prepare(&mut self, patches: Vec<(u8, Option<String>)>) {
        self.adaptive_buffer = None;
        // サーバー起動（CLAP インスタンス生成）と音色ロードは所要時間の桁が
        // 違うので、別々に計測してどちらが支配的かを切り分けられるようにする。
        let server_started = Instant::now();
        let ensure = self.supervisor.ensure_started_for_fast_midi();
        let server_elapsed = server_started.elapsed();
        let patch_started = Instant::now();
        let status = Arc::clone(&self.status);
        let result = ensure.and_then(|()| {
            status
                .lock()
                .unwrap()
                .begin_patch_setting(patches.len(), server_elapsed);
            prepare_instances(self.supervisor.as_ref(), &patches, |completed, total| {
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
        self.status
            .lock()
            .unwrap()
            .finish_patch_setting(patch_elapsed);
        if result.is_ok() {
            let now = Instant::now();
            let buffer = AdaptiveBuffer::new(now, self.supervisor.underrun_frames());
            self.status
                .lock()
                .unwrap()
                .update_adaptive_buffer(buffer.multiplier(), buffer.underrun_frames());
            self.adaptive_buffer = Some(buffer);
        }
        apply(
            &self.status,
            result.map(|()| self.supervisor.limiter_meter()),
            Some(server_elapsed + patch_elapsed),
            false,
        );
    }

    fn set_row_patch(
        &mut self,
        request_id: u64,
        queued_at: Instant,
        reason: &'static str,
        row: usize,
        instance_id: u8,
        patch: Option<String>,
    ) {
        let queue_ms = queued_at.elapsed().as_millis();
        let current = self
            .adaptive_buffer
            .map(AdaptiveBuffer::multiplier)
            .unwrap_or(INITIAL_BUFFER_MULTIPLIER);
        let _ = self
            .supervisor
            .set_connected_live_buffer_multiplier(current.max(ROW_PATCH_BUFFER_MULTIPLIER));
        let started = Instant::now();
        let result = self
            .supervisor
            .prepare_live_patch(instance_id, patch.as_deref());
        let _ = self
            .supervisor
            .set_connected_live_buffer_multiplier(current);
        let error = result.as_ref().err().map(|error| format!("{error:#}"));
        crate::log_line(&format!(
            "grid-sequencer: instance-patch request={request_id} reason={reason} logical_instance={} \
             server_instance={instance_id} patch={patch:?} queue_ms={queue_ms} load_ms={} result={}",
            row + 1,
            started.elapsed().as_millis(),
            if result.is_ok() { "ok" } else { "error" },
        ));
        self.status
            .lock()
            .unwrap()
            .finish_row_patch_setting(row, error);
    }

    fn set_gains(&mut self, gains: Vec<f32>) {
        let mut failed = None;
        for (instance_id, gain) in gains.iter().enumerate() {
            // 古いサーバーはこのコマンドを知らない。音量差が付かないだけなので
            // ログにとどめ、再生は続ける。
            if let Err(error) = self
                .supervisor
                .set_live_instance_gain(instance_id as u8, *gain)
            {
                failed = Some(format!("{error:#}"));
                break;
            }
        }
        crate::log_line(&format!(
            "grid-sequencer: gains instances={} adjusted={} result={}",
            gains.len(),
            describe_adjusted(&gains),
            match &failed {
                Some(error) => format!("error \"{error}\""),
                None => "ok".to_string(),
            },
        ));
    }

    fn set_auto_gain(&mut self, enabled: bool) {
        let result = self.supervisor.set_live_auto_gain_enabled(enabled);
        crate::log_line(&format!(
            "grid-sequencer: auto-gain enabled={enabled} result={}",
            match &result {
                Ok(()) => "ok".to_string(),
                Err(error) => format!("error \"{error:#}\""),
            },
        ));
    }

    fn stop(&mut self) {
        self.adaptive_buffer = None;
        // 画面を離れるので判定も白紙へ戻す。次に入るときはダブルバッファリングから。
        self.overload = OverloadDetector::new();
        self.status.lock().unwrap().clear_overload();
        let started = Instant::now();
        let result = self
            .supervisor
            .stop_live_all()
            .and_then(|()| {
                self.supervisor
                    .set_connected_live_buffer_multiplier(RESTORE_BUFFER_MULTIPLIER)
            })
            .map(|()| LimiterMeter::default());
        self.status
            .lock()
            .unwrap()
            .update_adaptive_buffer(RESTORE_BUFFER_MULTIPLIER, 0);
        apply(&self.status, result, Some(started.elapsed()), true);
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
