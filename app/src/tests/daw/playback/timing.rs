use super::*;

#[test]
fn current_play_measure_index_wraps_to_loop_start_when_measure_count_shrinks() {
    assert_eq!(current_play_measure_index(7, 4), 0);
    assert_eq!(current_play_measure_index(2, 4), 2);
}

#[test]
fn following_measure_index_wraps_after_last_measure() {
    assert_eq!(following_measure_index(1, 4), 2);
    assert_eq!(following_measure_index(3, 4), 0);
}

#[test]
fn format_playback_measure_resolution_log_shows_cursor_and_resolved_measure() {
    assert_eq!(
        format_playback_measure_resolution_log(7, 0, 4),
        "play: sync resolve cursor=meas8 -> current=meas1 (effective_count=4)"
    );
}

#[test]
fn format_playback_measure_advance_log_shows_current_and_next_measure() {
    assert_eq!(
        format_playback_measure_advance_log(1, 2, 4),
        "play: sync advance current=meas2 -> next=meas3 (effective_count=4)"
    );
}

#[test]
fn future_chunk_append_deadline_uses_50ms_margin_before_next_measure() {
    let measure_start = Instant::now();
    let deadline = future_chunk_append_deadline(
        measure_start,
        Duration::from_millis(400),
        Duration::from_millis(50),
    );

    assert_eq!(
        deadline.duration_since(measure_start),
        Duration::from_millis(350)
    );
}

#[test]
fn future_chunk_append_deadline_clamps_to_measure_start_for_short_measures() {
    let measure_start = Instant::now();
    let deadline = future_chunk_append_deadline(
        measure_start,
        Duration::from_millis(30),
        Duration::from_millis(50),
    );

    assert_eq!(deadline, measure_start);
}

#[test]
fn format_playback_future_append_log_reports_append_lead_time() {
    let append_time = Instant::now();
    let measure_start = append_time + Duration::from_millis(48);

    assert_eq!(
        format_playback_future_append_log(2, append_time, measure_start, Duration::from_millis(50),),
        "play: queue meas3 append lead=48ms (target_margin=50ms)"
    );
}

#[test]
fn format_playback_future_append_log_reports_late_append() {
    let measure_start = Instant::now();
    let append_time = measure_start + Duration::from_millis(12);

    assert_eq!(
        format_playback_future_append_log(2, append_time, measure_start, Duration::from_millis(50),),
        "play: queue meas3 append late=12ms (target_margin=50ms)"
    );
}

#[test]
fn resolved_measure_start_after_append_keeps_expected_start_when_append_is_early() {
    let expected_measure_start = Instant::now() + Duration::from_millis(50);
    let append_time = expected_measure_start - Duration::from_millis(12);

    assert_eq!(
        resolved_measure_start_after_append(expected_measure_start, append_time),
        expected_measure_start
    );
}

#[test]
fn resolved_measure_start_after_append_resyncs_to_late_append_time() {
    let expected_measure_start = Instant::now();
    let append_time = expected_measure_start + Duration::from_millis(12);

    assert_eq!(
        resolved_measure_start_after_append(expected_measure_start, append_time),
        append_time
    );
}

#[test]
fn wait_until_or_stop_returns_false_when_playback_is_not_running() {
    let play_state = Arc::new(Mutex::new(DawPlayState::Idle));

    assert!(!wait_until_or_stop(
        &play_state,
        Instant::now() + Duration::from_millis(50)
    ));
}

#[test]
fn wait_until_or_stop_returns_true_when_deadline_is_already_reached() {
    let play_state = Arc::new(Mutex::new(DawPlayState::Playing));

    assert!(wait_until_or_stop(
        &play_state,
        Instant::now() - Duration::from_millis(1)
    ));
}
