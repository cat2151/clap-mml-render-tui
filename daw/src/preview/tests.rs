use super::preview_render_progress_log_line;
use crate::preview::render::{
    render_mixed_preview_tracks, MixedPreviewRenderRequest, PreviewRenderProgress,
    PreviewRenderProgressPhase,
};
use crate::render_queue::{RenderPriority, RenderQueue};

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
            auto_trim: false,
        },
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
