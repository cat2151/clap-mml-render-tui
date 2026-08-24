use super::*;

#[test]
fn progress_knows_the_total_before_the_first_load_and_updates_eta() {
    let mut status = GridConnectionStatus::default();
    status.reset_preload(vec![100, 400, 200]);

    assert_eq!(
        status.preload,
        GridProgress {
            completed: 0,
            total: 3
        }
    );
    assert_eq!(status.preload_current_instance(), None);
    assert_eq!(status.preload_eta(), Some(Duration::from_millis(700)));

    status.begin_preload_step();
    assert_eq!(status.preload_current_instance(), Some(1));
    status.record_preload_step(true, Duration::from_millis(50));
    assert_eq!(status.preload.completed, 1);
    assert_eq!(status.preload_current_instance(), None);
    assert_eq!(status.preload_eta(), Some(Duration::from_millis(300)));
}

#[test]
fn cancelling_an_incomplete_preload_clears_the_loading_status() {
    let mut status = GridConnectionStatus::default();
    status.reset_preload(vec![100, 400]);
    status.begin_preload_step();

    status.finish_preload();

    assert_eq!(status.preload, GridProgress::default());
    assert_eq!(status.preload_estimate, None);
}
