use super::*;

#[test]
fn catalog_ratios_are_recalibrated_with_completed_patch_times() {
    let mut estimate = GridPreloadEstimate::new(vec![100, 400, 200]);
    let now = Instant::now();

    assert_eq!(estimate.current_instance(), None);
    assert_eq!(estimate.eta(now), Duration::from_millis(700));

    estimate.begin_step(now);
    assert_eq!(estimate.current_instance(), Some(1));
    assert_eq!(
        estimate.eta(now + Duration::from_millis(20)),
        Duration::from_millis(680)
    );
    assert!(estimate.record_step(Duration::from_millis(50)));
    assert_eq!(estimate.current_instance(), None);
    // catalog 100ms が実測50msだったので、残り600ms相当は300msと見積もる。
    assert_eq!(estimate.eta(now), Duration::from_millis(300));

    estimate.begin_step(now);
    assert_eq!(estimate.current_instance(), Some(2));
    assert!(estimate.record_step(Duration::from_millis(800)));
    // 完了済み重み500ms : 実測850ms の比で、残り200ms相当を換算する。
    assert_eq!(estimate.current_instance(), None);
    assert_eq!(estimate.eta(now), Duration::from_millis(340));
}

#[test]
fn zero_weights_and_duplicate_completions_cannot_break_progress() {
    let mut estimate = GridPreloadEstimate::new(vec![0]);
    let now = Instant::now();

    assert!(!estimate.record_step(Duration::from_secs(1)));
    estimate.begin_step(now);
    assert!(estimate.record_step(Duration::ZERO));
    assert!(!estimate.record_step(Duration::from_secs(1)));
    assert_eq!(estimate.completed(), 1);
    assert_eq!(estimate.current_instance(), None);
    assert_eq!(estimate.eta(now), Duration::ZERO);
}
