use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::PreviewOutputHandle;
use crate::{DawPlayState, PlayPosition};

struct OutputFixture {
    output: PreviewOutputHandle,
    transition_lock: Arc<Mutex<()>>,
    play_state: Arc<Mutex<DawPlayState>>,
    position: Arc<Mutex<Option<PlayPosition>>>,
}

fn output_fixture() -> OutputFixture {
    let transition_lock = Arc::new(Mutex::new(()));
    let play_state = Arc::new(Mutex::new(DawPlayState::Idle));
    let position = Arc::new(Mutex::new(None));
    OutputFixture {
        output: PreviewOutputHandle::new(
            Arc::clone(&transition_lock),
            Arc::clone(&play_state),
            Arc::clone(&position),
        ),
        transition_lock,
        play_state,
        position,
    }
}

#[test]
fn stale_session_finish_does_not_clear_current_session() {
    let OutputFixture {
        output, play_state, ..
    } = output_fixture();
    let stale = output.start_session();
    let current = output.start_session();

    assert!(!output.finish_session(stale));
    assert!(matches!(*play_state.lock().unwrap(), DawPlayState::Preview));
    assert!(output.is_current(current));
}

#[test]
fn only_current_session_finish_returns_preview_to_idle() {
    let OutputFixture {
        output,
        play_state,
        position,
        ..
    } = output_fixture();
    let current = output.start_session();
    assert!(output.enqueue_if_current(current, 3, Duration::from_secs(1), None, || {}));

    assert!(output.finish_session(current));
    assert!(matches!(*play_state.lock().unwrap(), DawPlayState::Idle));
    assert!(position.lock().unwrap().is_none());
}

#[test]
fn stale_session_samples_are_not_enqueued() {
    let OutputFixture {
        output, position, ..
    } = output_fixture();
    let stale = output.start_session();
    let _current = output.start_session();
    let enqueue_calls = AtomicUsize::new(0);

    assert!(
        !output.enqueue_if_current(stale, 2, Duration::from_secs(1), None, || {
            enqueue_calls.fetch_add(1, Ordering::SeqCst);
        })
    );
    assert_eq!(enqueue_calls.load(Ordering::SeqCst), 0);
    assert!(position.lock().unwrap().is_none());
}

#[test]
fn enqueue_callback_runs_without_transition_or_state_lock() {
    let OutputFixture {
        output,
        transition_lock,
        play_state,
        position,
    } = output_fixture();
    let current = output.start_session();
    let locks_were_free = AtomicBool::new(false);

    assert!(
        output.enqueue_if_current(current, 4, Duration::from_secs(2), None, || {
            let _transition = transition_lock
                .try_lock()
                .expect("enqueue callback must not hold transition lock");
            let _state = play_state
                .try_lock()
                .expect("enqueue callback must not hold state lock");
            locks_were_free.store(true, Ordering::SeqCst);
        })
    );

    assert!(locks_were_free.load(Ordering::SeqCst));
    let position = position.lock().unwrap();
    assert_eq!(position.as_ref().map(|value| value.measure_index), Some(4));
    assert_eq!(
        position.as_ref().map(|value| value.measure_duration),
        Some(Duration::from_secs(2))
    );
}
