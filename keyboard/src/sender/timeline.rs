//! worker 側の live timeline 送信と、予約を送っている間だけ出すタイミング統計のログ。

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::Result;
use cmrt_realtime_play::{
    LiveTimelineConfig, RealtimePlayServerSupervisor, TimelineId, TimelineMidiEvent,
};

use crate::logging::log_line;

/// timing 統計をログへ書き出す間隔。サーバーの統計もこの周期でしか更新されない。
const TIMING_LOG_INTERVAL: Duration = Duration::from_secs(5);
/// timeline は秒で予約するので tempo は使わない。サーバーが要求する値を埋めるだけ。
const TIMELINE_TEMPO_BPM: f64 = 120.0;
const TIME_SIGNATURE_NUMERATOR: u16 = 4;
const TIME_SIGNATURE_DENOMINATOR: u16 = 4;
/// サーバーは `timeline_id` の一致だけで予約を受け付ける。MML overlay・grid sequencer も
/// 1 から数えるので、この画面の id は桁を離して、他者の timeline へ紛れ込まないようにする。
const KEYBOARD_TIMELINE_ID_BASE: TimelineId = 1 << 40;

static NEXT_TIMELINE_ID: AtomicU64 = AtomicU64::new(KEYBOARD_TIMELINE_ID_BASE);

pub(crate) fn next_timeline_id() -> TimelineId {
    NEXT_TIMELINE_ID.fetch_add(1, Ordering::Relaxed)
}

pub(super) fn timeline_config(timeline_id: TimelineId, sample_rate_hz: f64) -> LiveTimelineConfig {
    LiveTimelineConfig {
        timeline_id,
        sample_rate_hz,
        tempo_bpm: TIMELINE_TEMPO_BPM,
        time_signature_numerator: TIME_SIGNATURE_NUMERATOR,
        time_signature_denominator: TIME_SIGNATURE_DENOMINATOR,
    }
}

#[derive(Default)]
pub(super) struct TimelineWorker {
    /// 予約を送った timeline。張り直した直後と停止後は `None` で、ログも出さない。
    sending: Option<TimelineId>,
    /// Begin 時点の late 累計。ログへ出すのはそこからの増分だけ。
    late_baseline: u64,
    last_log: Option<Instant>,
}

impl TimelineWorker {
    pub(super) fn begin(
        &mut self,
        supervisor: &RealtimePlayServerSupervisor,
        config: LiveTimelineConfig,
    ) -> Result<()> {
        let result = supervisor.begin_live_timeline(config);
        log_line(&format!(
            "timeline begin id={} sample_rate={} result={}",
            config.timeline_id,
            config.sample_rate_hz,
            if result.is_ok() { "ok" } else { "error" },
        ));
        if result.is_ok() {
            self.late_baseline = supervisor.timing_metrics().late_events_total;
        }
        self.sending = None;
        result
    }

    pub(super) fn send(
        &mut self,
        supervisor: &RealtimePlayServerSupervisor,
        events: &[TimelineMidiEvent],
        now: Instant,
    ) -> Result<()> {
        let result = supervisor.send_timeline_events(events).map(|_| ());
        if let (Ok(()), Some(event)) = (&result, events.first()) {
            if self.sending != Some(event.timeline_id) {
                self.sending = Some(event.timeline_id);
                self.last_log = Some(now);
            }
        }
        result
    }

    pub(super) fn stop(&mut self) {
        self.sending = None;
    }

    /// 予約を送っている間だけ、5 秒に 1 行 timing を書く。
    pub(super) fn poll(&mut self, supervisor: &RealtimePlayServerSupervisor, now: Instant) {
        let Some(timeline_id) = self.sending else {
            return;
        };
        if self
            .last_log
            .is_some_and(|last| now.saturating_duration_since(last) < TIMING_LOG_INTERVAL)
        {
            return;
        }
        let timing = supervisor.timing_metrics();
        log_line(&format!(
            "timing window_events={} late={}/{} late_max_samples={} late_max_us={:.1} \
             lead_frames={}..{} timeline={timeline_id}",
            timing.events,
            timing.late_events,
            timing.late_events_total.saturating_sub(self.late_baseline),
            timing.max_late_samples,
            timing.max_late_us,
            timing.output_lead_min_frames,
            timing.output_lead_max_frames,
        ));
        self.last_log = Some(now);
    }
}
