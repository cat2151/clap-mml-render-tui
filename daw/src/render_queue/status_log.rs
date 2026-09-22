//! render queue の詰まり具合（preview の cache render がどこまで進んだか）を 1 秒ごとに log へ出す。

use std::time::{Duration, Instant};

use super::RenderQueueSnapshot;
use crate::OVERLAY_PREVIEW_CACHE_MAX_ENTRIES;

const LOG_INTERVAL: Duration = Duration::from_secs(1);

/// 仕掛かり中の件数があるあいだは 1 秒ごとに出し、全部はけたら最後に 1 行出して止まる
/// （idle 中は同じ行を繰り返さない）。
#[derive(Default)]
pub(crate) struct RenderQueueStatusLog {
    last_logged_at: Option<Instant>,
    last_snapshot: RenderQueueSnapshot,
    last_overlay_cache_entries: usize,
}

impl RenderQueueStatusLog {
    /// 出すべきタイミングなら log 1 行を返す。
    pub(crate) fn line_if_due(
        &mut self,
        now: Instant,
        snapshot: RenderQueueSnapshot,
        overlay_cache_entries: usize,
    ) -> Option<String> {
        let due = self
            .last_logged_at
            .is_none_or(|at| now.duration_since(at) >= LOG_INTERVAL);
        if !due {
            return None;
        }
        let unchanged = snapshot == self.last_snapshot
            && overlay_cache_entries == self.last_overlay_cache_entries;
        if snapshot.in_flight() == 0 && unchanged {
            return None;
        }
        self.last_logged_at = Some(now);
        self.last_snapshot = snapshot;
        self.last_overlay_cache_entries = overlay_cache_entries;
        Some(status_line(snapshot, overlay_cache_entries))
    }
}

fn status_line(snapshot: RenderQueueSnapshot, overlay_cache_entries: usize) -> String {
    format!(
        "render-queue: waiting_high={} waiting_normal={} waiting_low={} rendering={} \
         completed={} failed={} overlay_preview_cache={}/{}",
        snapshot.waiting_high,
        snapshot.waiting_normal,
        snapshot.waiting_low,
        snapshot.rendering,
        snapshot.completed,
        snapshot.failed,
        overlay_cache_entries,
        OVERLAY_PREVIEW_CACHE_MAX_ENTRIES,
    )
}

#[cfg(test)]
mod tests;
