use std::time::{Duration, Instant};

use super::super::RenderQueueSnapshot;
use super::RenderQueueStatusLog;

fn busy() -> RenderQueueSnapshot {
    RenderQueueSnapshot {
        waiting_high: 1,
        waiting_low: 3,
        rendering: 4,
        completed: 10,
        ..RenderQueueSnapshot::default()
    }
}

#[test]
fn logs_every_second_while_in_flight() {
    let mut log = RenderQueueStatusLog::default();
    let t0 = Instant::now();
    let line = log.line_if_due(t0, busy(), 2).expect("first tick logs");
    assert_eq!(
        line,
        "render-queue: waiting_high=1 waiting_normal=0 waiting_low=3 rendering=4 \
         completed=10 failed=0 overlay_preview_cache=2/64"
    );
    assert!(log
        .line_if_due(t0 + Duration::from_millis(500), busy(), 2)
        .is_none());
    assert!(log
        .line_if_due(t0 + Duration::from_secs(1), busy(), 2)
        .is_some());
}

#[test]
fn logs_once_after_draining_then_stays_quiet() {
    let mut log = RenderQueueStatusLog::default();
    let t0 = Instant::now();
    log.line_if_due(t0, busy(), 2);
    let drained = RenderQueueSnapshot {
        completed: 18,
        ..RenderQueueSnapshot::default()
    };
    assert!(log
        .line_if_due(t0 + Duration::from_secs(1), drained, 6)
        .is_some());
    assert!(log
        .line_if_due(t0 + Duration::from_secs(2), drained, 6)
        .is_none());
    assert!(log
        .line_if_due(t0 + Duration::from_secs(3), drained, 6)
        .is_none());
}

#[test]
fn idle_queue_with_nothing_done_yet_logs_nothing() {
    let mut log = RenderQueueStatusLog::default();
    assert!(log
        .line_if_due(Instant::now(), RenderQueueSnapshot::default(), 0)
        .is_none());
}
