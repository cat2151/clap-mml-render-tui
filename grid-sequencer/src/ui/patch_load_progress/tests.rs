use super::*;
use crate::{GridPreloadEstimate, GridProgress};
use std::time::Instant;

fn loading_status() -> GridConnectionStatus {
    let mut estimate = GridPreloadEstimate::new(vec![100, 400, 200]);
    let future = Instant::now() + Duration::from_secs(60);
    estimate.begin_step(future);
    estimate.record_step(Duration::from_millis(50));
    estimate.begin_step(future);
    GridConnectionStatus {
        preload: GridProgress {
            completed: 1,
            total: 3,
        },
        preload_estimate: Some(estimate),
        ..GridConnectionStatus::default()
    }
}

#[test]
fn loading_text_contains_current_instance_total_and_weighted_eta() {
    let (text, color) = progress_text(&loading_status(), 80);

    assert!(text.contains("loading instance 2/3"), "{text}");
    assert!(text.contains("ETA 0.3s"), "{text}");
    assert_eq!(color, MONOKAI_YELLOW);
}

#[test]
fn narrow_text_keeps_the_instance_count_and_eta() {
    let (text, _) = progress_text(&loading_status(), 40);

    assert!(text.contains("inst 2/3"), "{text}");
    assert!(text.contains("ETA 0.3s"), "{text}");
    assert!(text.chars().count() <= 40, "{text}");
}

#[test]
fn eta_format_keeps_short_waits_precise_and_long_waits_readable() {
    assert_eq!(format_eta(Duration::from_millis(12_340)), "12.3s");
    assert_eq!(format_eta(Duration::from_millis(65_400)), "1m 05s");
}
