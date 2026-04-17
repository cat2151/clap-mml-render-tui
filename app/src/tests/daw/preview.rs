use std::sync::{
    atomic::AtomicU64,
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use super::{begin_preview_output, DawPlayState};
use crate::daw::PlayPosition;

#[test]
fn begin_preview_output_skips_enqueue_when_preview_stopped() {
    let play_transition_lock = Arc::new(Mutex::new(()));
    let play_state = Arc::new(Mutex::new(DawPlayState::Idle));
    let play_position = Arc::new(Mutex::new(None::<PlayPosition>));
    let preview_session = AtomicU64::new(1);
    let enqueue_calls = Arc::new(AtomicUsize::new(0));

    let started = begin_preview_output(
        &play_transition_lock,
        &play_state,
        &play_position,
        &preview_session,
        1,
        2,
        || {
            enqueue_calls.fetch_add(1, Ordering::SeqCst);
        },
    );

    assert!(!started);
    assert_eq!(enqueue_calls.load(Ordering::SeqCst), 0);
    assert!(play_position.lock().unwrap().is_none());
}

#[test]
fn begin_preview_output_updates_position_before_enqueue() {
    let play_transition_lock = Arc::new(Mutex::new(()));
    let play_state = Arc::new(Mutex::new(DawPlayState::Preview));
    let play_position = Arc::new(Mutex::new(None::<PlayPosition>));
    let preview_session = AtomicU64::new(4);
    let observed_measure = Arc::new(Mutex::new(None));

    let started = begin_preview_output(
        &play_transition_lock,
        &play_state,
        &play_position,
        &preview_session,
        4,
        3,
        {
            let play_position = Arc::clone(&play_position);
            let observed_measure = Arc::clone(&observed_measure);
            move || {
                *observed_measure.lock().unwrap() = play_position
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|position| position.measure_index);
            }
        },
    );

    assert!(started);
    assert_eq!(*observed_measure.lock().unwrap(), Some(3));
    assert_eq!(
        play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index),
        Some(3)
    );
}

#[test]
fn begin_preview_output_skips_enqueue_for_stale_preview_session() {
    let play_transition_lock = Arc::new(Mutex::new(()));
    let play_state = Arc::new(Mutex::new(DawPlayState::Preview));
    let play_position = Arc::new(Mutex::new(None::<PlayPosition>));
    let preview_session = AtomicU64::new(2);
    let enqueue_calls = Arc::new(AtomicUsize::new(0));

    let started = begin_preview_output(
        &play_transition_lock,
        &play_state,
        &play_position,
        &preview_session,
        1,
        2,
        || {
            enqueue_calls.fetch_add(1, Ordering::SeqCst);
        },
    );

    assert!(!started);
    assert_eq!(enqueue_calls.load(Ordering::SeqCst), 0);
    assert!(play_position.lock().unwrap().is_none());
}
