use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const SLOW_RENDER: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug)]
pub(in crate::tui) struct RenderMetrics {
    pub(in crate::tui) trace_id: Option<u64>,
    pub(in crate::tui) tree: Duration,
    pub(in crate::tui) tracks: Duration,
    pub(in crate::tui) pads: Duration,
    pub(in crate::tui) draw: Duration,
    pub(in crate::tui) rendered_tree_nodes: usize,
    pub(in crate::tui) total_tree_nodes: usize,
}

pub(super) struct PreviewMetrics<'a> {
    pub(super) trace_id: u64,
    pub(super) queue: Duration,
    pub(super) open: Duration,
    pub(super) decode: Duration,
    pub(super) sink: Duration,
    pub(super) append: Duration,
    pub(super) total: Duration,
    pub(super) path: &'a Path,
    pub(super) outcome: &'a str,
}

pub(in crate::tui) fn next_trace_id() -> u64 {
    static NEXT_TRACE_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_TRACE_ID.fetch_add(1, Ordering::Relaxed)
}

pub(super) fn log_cursor_move(
    trace_id: u64,
    elapsed: Duration,
    from: usize,
    to: usize,
    visible: usize,
    selected_kind: &str,
    preview: bool,
) {
    log(format!(
        "event=cursor-move trace={trace_id} slow={} input_ms={} from={from} to={to} visible={visible} selected={selected_kind} preview={preview}",
        elapsed >= SLOW_RENDER,
        millis(elapsed),
    ));
}

pub(super) fn log_preview_enqueued(trace_id: u64, elapsed: Duration, path: &Path, outcome: &str) {
    log(format!(
        "event=preview-enqueued trace={trace_id} slow={} send_ms={} outcome={outcome} path=\"{}\"",
        elapsed >= SLOW_RENDER,
        millis(elapsed),
        path_label(path),
    ));
}

pub(super) fn log_preview_finished(metrics: PreviewMetrics<'_>) {
    log(format!(
        "event=preview-finished trace={} slow={} queue_ms={} open_ms={} decode_ms={} sink_ms={} append_ms={} total_ms={} outcome={} path=\"{}\"",
        metrics.trace_id,
        metrics.total >= SLOW_RENDER,
        millis(metrics.queue),
        millis(metrics.open),
        millis(metrics.decode),
        millis(metrics.sink),
        millis(metrics.append),
        millis(metrics.total),
        metrics.outcome,
        path_label(metrics.path),
    ));
}

pub(in crate::tui) fn log_render(metrics: RenderMetrics, terminal_draw: Duration) {
    if metrics.trace_id.is_none() && terminal_draw < SLOW_RENDER {
        return;
    }
    let trace = metrics
        .trace_id
        .map_or_else(|| "none".to_string(), |trace| trace.to_string());
    log(format!(
        "event=render trace={trace} slow={} terminal_ms={} draw_ms={} tree_ms={} tracks_ms={} pads_ms={} tree_rendered={} tree_total={}",
        terminal_draw >= SLOW_RENDER,
        millis(terminal_draw),
        millis(metrics.draw),
        millis(metrics.tree),
        millis(metrics.tracks),
        millis(metrics.pads),
        metrics.rendered_tree_nodes,
        metrics.total_tree_nodes,
    ));
}

fn log(message: String) {
    crate::logging::append_global_log_line_nonblocking(format!("loop-browser-perf: {message}"));
}

fn millis(duration: Duration) -> String {
    format!("{:.3}", duration.as_secs_f64() * 1_000.0)
}

fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millisecond_format_keeps_sub_millisecond_detail() {
        assert_eq!(millis(Duration::from_micros(125)), "0.125");
    }
}
