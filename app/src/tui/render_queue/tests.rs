use super::*;

fn prefetch_waiter(sequence: u64) -> TuiRenderWaiter {
    prefetch_waiter_with_generation(sequence, 1)
}

fn prefetch_waiter_with_generation(sequence: u64, generation: u64) -> TuiRenderWaiter {
    let (response_tx, _response_rx) = mpsc::channel();
    TuiRenderWaiter {
        sequence,
        kind: TuiRenderWaiterKind::Prefetch { generation },
        response_tx,
    }
}

fn playback_waiter(
    sequence: u64,
    session: u64,
    playback_session: Arc<AtomicU64>,
) -> TuiRenderWaiter {
    let (response_tx, _response_rx) = mpsc::channel();
    TuiRenderWaiter {
        sequence,
        kind: TuiRenderWaiterKind::Playback {
            session,
            playback_session,
        },
        response_tx,
    }
}

#[test]
fn queue_stats_count_pending_unique_jobs_and_pending_playback_jobs() {
    let playback_session = Arc::new(AtomicU64::new(1));
    let mut state = TuiRenderQueueState::default();

    state.push_waiter("prefetch".to_string(), prefetch_waiter(1));
    state.push_waiter(
        "playback".to_string(),
        playback_waiter(2, 1, Arc::clone(&playback_session)),
    );
    state.push_waiter(
        "playback".to_string(),
        playback_waiter(3, 1, Arc::clone(&playback_session)),
    );

    assert_eq!(
        state.stats(2),
        TuiRenderQueueStats {
            workers: 2,
            pending_jobs: 2,
            pending_playback_jobs: 1,
        }
    );
}

#[test]
fn render_worker_count_caps_tui_workers_at_two() {
    assert_eq!(render_worker_count(0), 1);
    assert_eq!(render_worker_count(1), 1);
    assert_eq!(render_worker_count(2), 2);
    assert_eq!(render_worker_count(3), 2);
    assert_eq!(render_worker_count(4), 2);
}

#[test]
fn pending_queue_prefers_playback_before_prefetch() {
    let playback_session = Arc::new(AtomicU64::new(1));
    let mut state = TuiRenderQueueState::default();

    state.push_waiter("prefetch".to_string(), prefetch_waiter(1));
    state.push_waiter(
        "playback".to_string(),
        playback_waiter(2, 1, Arc::clone(&playback_session)),
    );

    assert!(state.drain_stale_pending_playback_waiters().is_empty());
    assert_eq!(state.next_pending_key().as_deref(), Some("playback"));
}

#[test]
fn pending_queue_deduplicates_same_mml_and_elevates_to_playback() {
    let playback_session = Arc::new(AtomicU64::new(1));
    let mut state = TuiRenderQueueState::default();

    state.push_waiter("same".to_string(), prefetch_waiter(1));
    state.push_waiter(
        "same".to_string(),
        playback_waiter(3, 1, Arc::clone(&playback_session)),
    );

    assert_eq!(state.jobs.len(), 1);
    let job = state.jobs.get("same").unwrap();
    assert_eq!(job.waiters.len(), 2);
    assert_eq!(
        job.effective_priority_sequence(),
        Some((TuiRenderPriority::Playback, 3))
    );
}

#[test]
fn newer_prefetch_generation_runs_before_older_prefetch() {
    let mut state = TuiRenderQueueState::default();

    state.push_waiter("old".to_string(), prefetch_waiter_with_generation(1, 1));
    state.push_waiter("new".to_string(), prefetch_waiter_with_generation(2, 2));

    assert_eq!(state.next_pending_key().as_deref(), Some("new"));
}

#[test]
fn stale_playback_waiters_are_dropped_before_render_selection() {
    let playback_session = Arc::new(AtomicU64::new(2));
    let mut state = TuiRenderQueueState::default();

    state.push_waiter(
        "old".to_string(),
        playback_waiter(1, 1, Arc::clone(&playback_session)),
    );
    state.push_waiter("prefetch".to_string(), prefetch_waiter(2));

    let stale = state.drain_stale_pending_playback_waiters();

    assert_eq!(stale.len(), 1);
    assert!(!state.jobs.contains_key("old"));
    assert_eq!(state.next_pending_key().as_deref(), Some("prefetch"));
}
