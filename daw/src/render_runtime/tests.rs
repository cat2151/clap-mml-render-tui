use std::sync::Arc;
use std::sync::{mpsc, Barrier};
use std::time::Duration;
use std::time::Instant;

use super::DawRenderRuntime;
use crate::render_queue::RenderQueueSnapshot;

#[test]
fn disabled_runtime_snapshot_is_idle() {
    let runtime = DawRenderRuntime::disabled_for_tests();

    assert_eq!(
        runtime.queue_handle().snapshot(),
        RenderQueueSnapshot::default()
    );
}

#[test]
fn status_line_reports_preview_cache_entry_count() {
    let mut runtime = DawRenderRuntime::disabled_for_tests();
    runtime.preview_cache().insert(42, 0, Arc::new(Vec::new()));

    let line = runtime
        .status_line_if_due(Instant::now())
        .expect("cache entry count change should be logged");

    assert!(line.contains("overlay_preview_cache=1/64"));
}

#[test]
fn clear_preview_cache_resets_entry_count() {
    let runtime = DawRenderRuntime::disabled_for_tests();
    let cache = runtime.preview_cache();
    cache.insert(42, 0, Arc::new(Vec::new()));

    runtime.clear_preview_cache();

    assert_eq!(cache.entry_count(), 0);
}

#[test]
fn status_snapshot_returns_while_preview_cache_map_lock_is_held() {
    let mut runtime = DawRenderRuntime::disabled_for_tests();
    let cache = runtime.preview_cache();
    let cache_lock_acquired = Arc::new(Barrier::new(2));
    let (release_tx, release_rx) = mpsc::channel();
    let lock_thread = {
        let cache_lock_acquired = Arc::clone(&cache_lock_acquired);
        std::thread::spawn(move || {
            cache.with_entries_lock_held_for_test(|| {
                cache_lock_acquired.wait();
                release_rx.recv().unwrap();
            });
        })
    };
    cache_lock_acquired.wait();

    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    let snapshot_thread = std::thread::spawn(move || {
        let line = runtime.status_line_if_due(Instant::now());
        snapshot_tx.send(line).unwrap();
    });

    let snapshot_while_locked = snapshot_rx.recv_timeout(Duration::from_secs(1));
    release_tx.send(()).unwrap();
    lock_thread.join().unwrap();
    snapshot_thread.join().unwrap();
    assert!(
        snapshot_while_locked.is_ok(),
        "status snapshot must not wait for the preview cache map lock"
    );
}
