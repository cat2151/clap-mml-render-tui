//! auto random patch の進捗欄が、通常画面の所定位置に描かれることを確かめる。

use super::*;
use crate::GridPreloadEstimate;

#[test]
fn the_patch_load_row_shows_instance_total_and_eta_below_the_tracks() {
    let screen = GridSequencerScreen::new(None);
    let mut estimate = GridPreloadEstimate::new(vec![100, 400, 200]);
    let future = Instant::now() + Duration::from_secs(60);
    estimate.begin_step(future);
    estimate.record_step(Duration::from_millis(50));
    estimate.begin_step(future);
    let connection = GridConnectionStatus {
        preload: GridProgress {
            completed: 1,
            total: 3,
        },
        preload_estimate: Some(estimate),
        ..GridConnectionStatus::default()
    };

    let rendered = render_with_connection(&screen, &connection);
    let layout = test_layout(&screen);
    let progress_line = rendered
        .lines()
        .nth(usize::from(layout.patch_load_progress.y))
        .expect("progress row is inside the terminal");

    assert!(progress_line.contains("AUTO PATCH LOAD"), "{rendered}");
    assert!(progress_line.contains("instance 2/3"), "{rendered}");
    assert!(progress_line.contains("ETA 0.3s"), "{rendered}");
}
