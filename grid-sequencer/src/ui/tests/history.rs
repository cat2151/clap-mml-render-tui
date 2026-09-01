use super::*;
use crate::GridHistoryPreviewStatus;

#[test]
fn grid_history_overlay_shows_the_recorded_loop_and_import_action() {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::with_track_count(None, 2);
    screen.state.start(now);
    screen.state.poll_steps(now, Duration::ZERO);
    screen.absorb_history_snapshots();
    screen.history.open(false);

    let rendered = render(&screen);

    assert!(rendered.contains("Grid History"), "{rendered}");
    assert!(rendered.contains("#0001"), "{rendered}");
    assert!(rendered.contains("select+play"), "{rendered}");
    assert!(rendered.contains("Space:stop/replay"), "{rendered}");
    assert!(rendered.contains("Preview:"), "{rendered}");
    assert!(rendered.contains("Daily DAW"), "{rendered}");
}

#[test]
fn rendering_history_preview_has_a_dedicated_progress_overlay() {
    let mut screen = GridSequencerScreen::with_track_count(None, 7);
    screen.history.open(false);
    screen.set_history_preview_status(GridHistoryPreviewStatus::Rendering {
        completed: 5,
        total: 7,
    });

    let rendered = render(&screen);

    assert!(rendered.contains("History Preview Rendering"), "{rendered}");
    assert!(rendered.contains("render 5/7 tracks"), "{rendered}");
    assert!(rendered.contains("elapsed 0s"), "{rendered}");
}
