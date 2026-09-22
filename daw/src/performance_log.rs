//! DAW UI の長時間停止箇所を特定するための低頻度 performance log。
//!
//! 100ms を超えた区間だけを、app から注入された非同期 sink へ送る。

use std::time::{Duration, Instant};

const SLOW_OPERATION_THRESHOLD: Duration = Duration::from_millis(100);

pub(crate) struct SlowOperation {
    stage: &'static str,
    context: Option<String>,
    started_at: Instant,
}

impl SlowOperation {
    pub(crate) fn new(stage: &'static str) -> Self {
        Self {
            stage,
            context: None,
            started_at: Instant::now(),
        }
    }

    pub(crate) fn with_context(stage: &'static str, context: String) -> Self {
        Self {
            stage,
            context: Some(context),
            started_at: Instant::now(),
        }
    }
}

impl Drop for SlowOperation {
    fn drop(&mut self) {
        log_slow_elapsed(
            self.stage,
            self.started_at.elapsed(),
            self.context.as_deref(),
        );
    }
}

pub(crate) fn log_slow_since(stage: &'static str, started_at: Instant, context: Option<&str>) {
    log_slow_elapsed(stage, started_at.elapsed(), context);
}

fn log_slow_elapsed(stage: &'static str, elapsed: Duration, context: Option<&str>) {
    if let Some(line) = slow_operation_log_line(stage, elapsed, context) {
        crate::performance_log_line(&line);
    }
}

fn slow_operation_log_line(
    stage: &'static str,
    elapsed: Duration,
    context: Option<&str>,
) -> Option<String> {
    if elapsed <= SLOW_OPERATION_THRESHOLD {
        return None;
    }
    let elapsed_ms = elapsed.as_secs_f64() * 1_000.0;
    Some(match context {
        Some(context) => format!(
            "daw-performance: event=slow-operation stage={stage} elapsed_ms={elapsed_ms:.3} context={context:?}"
        ),
        None => format!(
            "daw-performance: event=slow-operation stage={stage} elapsed_ms={elapsed_ms:.3}"
        ),
    })
}

#[cfg(test)]
mod tests;
