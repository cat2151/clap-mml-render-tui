use std::sync::{
    atomic::AtomicU64,
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use super::{
    begin_preview_output, preview_render_progress_log_line, render_mixed_preview_tracks,
    DawPlayState, MixedPreviewRenderRequest, PreviewOutputRequest, PreviewOutputState,
};
use crate::preview::render::{PreviewRenderProgress, PreviewRenderProgressPhase};
use crate::render_queue::{RenderPriority, RenderQueue};
use crate::PlayPosition;
use cmrt_core::NativeRenderProbeContext;

#[test]
fn begin_preview_output_skips_enqueue_when_preview_stopped() {
    let play_transition_lock = Arc::new(Mutex::new(()));
    let play_state = Arc::new(Mutex::new(DawPlayState::Idle));
    let play_position = Arc::new(Mutex::new(None::<PlayPosition>));
    let preview_session = AtomicU64::new(1);
    let enqueue_calls = Arc::new(AtomicUsize::new(0));

    let started = begin_preview_output(
        PreviewOutputState {
            play_transition_lock: &play_transition_lock,
            play_state: &play_state,
            play_position: &play_position,
            preview_session: &preview_session,
        },
        PreviewOutputRequest {
            session: 1,
            measure_index: 2,
            measure_duration: std::time::Duration::from_secs(1),
        },
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
        PreviewOutputState {
            play_transition_lock: &play_transition_lock,
            play_state: &play_state,
            play_position: &play_position,
            preview_session: &preview_session,
        },
        PreviewOutputRequest {
            session: 4,
            measure_index: 3,
            measure_duration: std::time::Duration::from_secs(1),
        },
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
    assert_eq!(
        play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_duration),
        Some(std::time::Duration::from_secs(1))
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
        PreviewOutputState {
            play_transition_lock: &play_transition_lock,
            play_state: &play_state,
            play_position: &play_position,
            preview_session: &preview_session,
        },
        PreviewOutputRequest {
            session: 1,
            measure_index: 2,
            measure_duration: std::time::Duration::from_secs(1),
        },
        || {
            enqueue_calls.fetch_add(1, Ordering::SeqCst);
        },
    );

    assert!(!started);
    assert_eq!(enqueue_calls.load(Ordering::SeqCst), 0);
    assert!(play_position.lock().unwrap().is_none());
}

#[test]
fn preview_render_reports_started_and_error_progress() {
    let render_queue = RenderQueue::disabled_for_tests();
    let active_tracks = [crate::FIRST_PLAYABLE_TRACK];
    let mut track_mmls = vec![String::new(); crate::FIRST_PLAYABLE_TRACK + 1];
    track_mmls[crate::FIRST_PLAYABLE_TRACK] = "c".to_string();
    let track_gains = vec![1.0; track_mmls.len()];
    let mut progress = Vec::new();

    let result = render_mixed_preview_tracks(
        &render_queue,
        MixedPreviewRenderRequest {
            priority: RenderPriority::High,
            measure_samples: 16,
            active_tracks: &active_tracks,
            track_mmls: &track_mmls,
            track_gains: &track_gains,
        },
        |track, _| NativeRenderProbeContext::preview(track, 0, 1, 1, 1),
        |event| progress.push(event),
    );

    assert!(result.is_none());
    assert_eq!(progress.len(), 2);
    assert_eq!(
        progress[0],
        PreviewRenderProgress {
            track: crate::FIRST_PLAYABLE_TRACK,
            completed: 0,
            total: 1,
            phase: PreviewRenderProgressPhase::Started,
        }
    );
    assert!(matches!(
        progress[1],
        PreviewRenderProgress {
            completed: 1,
            total: 1,
            phase: PreviewRenderProgressPhase::Error { .. },
            ..
        }
    ));
}

#[test]
fn preview_render_progress_log_identifies_measure_track_and_elapsed_time() {
    assert_eq!(
        preview_render_progress_log_line(
            0,
            PreviewRenderProgress {
                track: crate::FIRST_PLAYABLE_TRACK,
                completed: 1,
                total: 2,
                phase: PreviewRenderProgressPhase::Done { elapsed_ms: 9_001 },
            },
        ),
        "preview: render progress meas1 1/2 track1 done ms=9001"
    );
}
