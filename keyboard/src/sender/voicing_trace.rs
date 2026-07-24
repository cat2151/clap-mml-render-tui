use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use anyhow::Result;

use super::{KeyboardTransport, PatchVoicing};
use cmrt_realtime_play::VoicingReport;

static NEXT_PROBE_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct VoicingTrace {
    id: u64,
    source: &'static str,
    queued_at: Instant,
}

impl VoicingTrace {
    pub(super) fn queued(
        source: &'static str,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        previous_patch: Option<&str>,
        patch: Option<&str>,
    ) -> Self {
        let trace = Self {
            id: NEXT_PROBE_ID.fetch_add(1, Ordering::Relaxed),
            source,
            queued_at: Instant::now(),
        };
        trace.write(format!(
            "event=queued transport={} buffer_multiplier={} previous_patch={previous_patch:?} patch={patch:?}",
            transport.label(),
            buffer_multiplier,
        ));
        trace
    }

    pub(super) fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn worker_started(
        &self,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        patch: Option<&str>,
    ) {
        self.write(format!(
            "event=worker-start queue_wait_ms={} transport={} buffer_multiplier={} patch={patch:?}",
            self.queued_at.elapsed().as_millis(),
            transport.label(),
            buffer_multiplier,
        ));
    }

    pub(super) fn status_apply(
        &self,
        transport: KeyboardTransport,
        patch: Option<&str>,
        result: &Result<Option<VoicingReport>>,
        known_voicing: Option<PatchVoicing>,
        elapsed_ms: u128,
    ) {
        let outcome = match (result, known_voicing) {
            (Ok(Some(report)), _) => format!("report={report:?}"),
            // probe を省いた場合も `Ok(None)` になるので、cache ヒットと
            // 旧サーバーへのフォールバックをログ上で区別する。
            (Ok(None), Some(voicing)) => format!("report={voicing:?} source=cache probe=skipped"),
            (Ok(None), None) => "report=unavailable fallback=legacy-server".to_string(),
            (Err(error), _) => format!("error={error:?}"),
        };
        self.write(format!(
            "event=status-apply elapsed_ms={elapsed_ms} transport={} patch={patch:?} {outcome}",
            transport.label(),
        ));
    }

    fn write(&self, message: String) {
        crate::log_line(&format!(
            "keyboard-voicing: probe_id={} source={} {message}",
            self.id, self.source
        ));
    }
}
