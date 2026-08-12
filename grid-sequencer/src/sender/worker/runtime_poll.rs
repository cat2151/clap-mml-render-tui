//! コマンドの空き時間に走る、サーバー状態の取り込みと出力バッファの適応調整。
//!
//! ここは「送る」側ではなく「見る」側。リミッターメーター・auto gain・タイミング統計を
//! status へ写し、underrun の増え方から出力バッファの厚さを上げ下げする。

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use super::super::{adaptive_buffer::AdaptiveBuffer, overload::OverloadDetector};
use super::GridConnectionStatus;

/// タイミング統計をログへ書き出す間隔。
const TIMING_LOG_INTERVAL: Duration = Duration::from_secs(5);

pub(super) struct RuntimePollContext<'a> {
    pub(super) supervisor: &'a RealtimePlayServerSupervisor,
    pub(super) status: &'a Mutex<GridConnectionStatus>,
    pub(super) adaptive_buffer: &'a mut Option<AdaptiveBuffer>,
    pub(super) overload: &'a mut OverloadDetector,
    pub(super) last_timing_log: &'a mut Instant,
    /// `BeginTimeline` 時点の late 累計。画面へ出すのはそこからの増分だけ。
    pub(super) timing_late_baseline: u64,
}

pub(super) fn poll_runtime_status(context: RuntimePollContext<'_>, now: Instant) {
    let RuntimePollContext {
        supervisor,
        status,
        adaptive_buffer,
        overload,
        last_timing_log,
        timing_late_baseline,
    } = context;
    absorb_metrics(
        supervisor,
        status,
        last_timing_log,
        timing_late_baseline,
        now,
    );
    if !status.lock().unwrap().phase.accepts_notes() {
        return;
    }
    let Some(buffer) = adaptive_buffer.as_mut() else {
        return;
    };
    // 梯子を上げる前の厚さで判定する。上げた直後に上限になっても、そのドロップは
    // 1段下での出来事なので数えない。
    let was_at_max = buffer.is_at_max();
    let adjustment = buffer.observe(now, supervisor.underrun_frames());
    status
        .lock()
        .unwrap()
        .record_underruns(buffer.last_new_underrun_frames());
    if overload.observe(now, was_at_max, buffer.last_new_underrun_frames()) {
        status.lock().unwrap().mark_overloaded();
        crate::log_line(&format!(
            "grid-sequencer: overload detected multiplier={} -> single buffering",
            buffer.multiplier()
        ));
    }
    if let Some(multiplier) = adjustment {
        apply_buffer_adjustment(supervisor, status, buffer, multiplier);
    }
    status
        .lock()
        .unwrap()
        .update_adaptive_buffer(buffer.multiplier(), buffer.underrun_frames());
}

fn absorb_metrics(
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<GridConnectionStatus>,
    last_timing_log: &mut Instant,
    timing_late_baseline: u64,
    now: Instant,
) {
    let mut status = status.lock().unwrap();
    status.update_limiter_meter(supervisor.limiter_meter());
    status.update_auto_gain_db(supervisor.live_auto_gain_db());
    let mut timing = supervisor.timing_metrics();
    timing.late_events_total = timing
        .late_events_total
        .saturating_sub(timing_late_baseline);
    status.update_timing(timing);
    if now.saturating_duration_since(*last_timing_log) < TIMING_LOG_INTERVAL {
        return;
    }
    let timing = status.timing;
    crate::log_line(&format!(
        "grid-sequencer: timing window_events={} late={}/{} late_max_samples={} \
         late_max_us={:.1} lead_frames={}..{} cpu_p95={:.0}% cpu_max={:.1}% \
         underrun_level={} underrun_total={} pump_late_max_us={} sender_queue_max_us={}",
        timing.events,
        timing.late_events,
        timing.late_events_total,
        timing.max_late_samples,
        timing.max_late_us,
        timing.output_lead_min_frames,
        timing.output_lead_max_frames,
        timing.process_load_p95,
        timing.process_load_max,
        status.underrun_frames,
        status.underrun_frames_total,
        status.pump_late_max_us,
        status.sender_queue_max_us,
    ));
    status.reset_sender_timing_window();
    *last_timing_log = now;
}

fn apply_buffer_adjustment(
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<GridConnectionStatus>,
    buffer: &mut AdaptiveBuffer,
    multiplier: u16,
) {
    let previous = status.lock().unwrap().buffer_multiplier;
    // 倍率の設定に失敗しても演奏は止めない。古いサーバーは新しい上限（x32 以上）を
    // 拒否するが、それは「バッファを厚くできない」だけで、鳴らせなくなる理由ではない。
    // phase を Error にすると accepts_notes() が落ちて無音になってしまう。
    if let Err(error) = supervisor.set_live_buffer_multiplier(multiplier) {
        buffer.revert(previous);
        crate::log_line(&format!(
            "grid-sequencer: buffer auto {previous} -> {multiplier} rejected error=\"{error:#}\""
        ));
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
