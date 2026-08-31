//! [`GridSenderBackend`] の実装。実際の play server とやり取りする側。
//!
//! ここに直接あるのは「止めてはいけない経路」（timeline 送出と先読みの受付/完了
//! ポーリング）だけ。完了まで待ってよいコマンドは [`slow_commands`] へ分けてある。

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_realtime_play::{
    LimiterMeter, RealtimePlayServerSupervisor, StandbyPatchRequest, TimelineMidiEvent,
};

use super::{
    super::{
        adaptive_buffer::AdaptiveBuffer, overload::OverloadDetector, GridConnectionStatus,
        GridMidiCommand,
    },
    preload::PreloadOutcome,
    runtime_poll::{poll_runtime_status, RuntimePollContext},
    GridSenderBackend,
};

mod slow_commands;

pub(super) struct SupervisorBackend {
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<GridConnectionStatus>>,
    adaptive_buffer: Option<AdaptiveBuffer>,
    /// 慢性ドロップの判定は `adaptive_buffer` の中に置かないこと。`Prepare` のたびに
    /// `adaptive_buffer = None` されるので、そこに持たせると latch が毎回消える。
    overload: OverloadDetector,
    last_timing_log: Instant,
    /// `BeginTimeline` 時点の late 累計。画面へ出すのはそこからの増分だけ。
    timing_late_baseline: u64,
}

impl SupervisorBackend {
    pub(super) fn new(
        supervisor: Arc<RealtimePlayServerSupervisor>,
        status: Arc<Mutex<GridConnectionStatus>>,
    ) -> Self {
        Self {
            supervisor,
            status,
            adaptive_buffer: None,
            overload: OverloadDetector::new(),
            last_timing_log: Instant::now(),
            timing_late_baseline: 0,
        }
    }
}

impl GridSenderBackend for SupervisorBackend {
    type Standby = StandbyPatchRequest;

    fn send_timeline(
        &mut self,
        events: &[TimelineMidiEvent],
        queued_at: Instant,
        pump_lateness: Duration,
    ) {
        self.status
            .lock()
            .unwrap()
            .observe_sender_timing(pump_lateness, queued_at.elapsed());
        let started = Instant::now();
        let result = self.supervisor.send_timeline_events(events);
        apply(&self.status, result, Some(started.elapsed()), false);
    }

    fn begin_standby(
        &mut self,
        instance_id: u8,
        patch: Option<&str>,
    ) -> anyhow::Result<Self::Standby> {
        self.supervisor.begin_standby_patch(instance_id, patch)
    }

    fn poll_standby(&mut self, request: &mut Self::Standby) -> anyhow::Result<Option<()>> {
        self.supervisor.poll_standby_patch(request)
    }

    fn abandon_standby(&mut self, request: Self::Standby) {
        self.supervisor.abandon_standby_patch(request);
    }

    fn standby_request_id(&self, request: &Self::Standby) -> u32 {
        request.request_id()
    }

    fn record_preload_outcome(&mut self, outcome: PreloadOutcome) {
        let request = outcome
            .request_id
            .map_or_else(|| "-".to_string(), |request_id| request_id.to_string());
        let ms = outcome.elapsed.as_millis();
        match (&outcome.error, outcome.stale) {
            // 畳んだサイクルの結果。ログには残すが進捗にも失敗にも数えない。
            (_, true) => crate::log_line(&format!(
                "grid-sequencer: preload stale instance={} request={request} ms={ms} result={}",
                outcome.instance_id,
                match &outcome.error {
                    Some(error) => format!("error \"{error}\""),
                    None => "ok".to_string(),
                },
            )),
            (None, false) => crate::log_line(&format!(
                "grid-sequencer: preload instance={} request={request} ms={ms}",
                outcome.instance_id
            )),
            (Some(error), false) => crate::log_line(&format!(
                "grid-sequencer: preload failed instance={} request={request} ms={ms} \
                 error=\"{error}\"",
                outcome.instance_id
            )),
        }
        if outcome.stale {
            return;
        }
        self.status
            .lock()
            .unwrap()
            .record_preload_step(outcome.error.is_none(), outcome.elapsed);
    }

    fn handle_slow_command(&mut self, command: GridMidiCommand) {
        self.dispatch_slow_command(command);
    }

    fn poll_runtime(&mut self, now: Instant) {
        poll_runtime_status(
            RuntimePollContext {
                supervisor: self.supervisor.as_ref(),
                status: &self.status,
                adaptive_buffer: &mut self.adaptive_buffer,
                overload: &mut self.overload,
                last_timing_log: &mut self.last_timing_log,
                timing_late_baseline: self.timing_late_baseline,
            },
            now,
        );
    }
}

/// 結果を [`GridConnectionStatus`] へ写す。hot path と slow command の両方が通る。
fn apply(
    status: &Mutex<GridConnectionStatus>,
    result: anyhow::Result<LimiterMeter>,
    elapsed: Option<Duration>,
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
