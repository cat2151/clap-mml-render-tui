use super::*;

#[test]
fn current_play_measure_index_wraps_to_loop_start_when_measure_count_shrinks() {
    assert_eq!(current_play_measure_index(7, 4, None), 0);
    assert_eq!(current_play_measure_index(2, 4, None), 2);
}

#[test]
fn following_measure_index_wraps_after_last_measure() {
    assert_eq!(following_measure_index(1, 4, None), 2);
    assert_eq!(following_measure_index(3, 4, None), 0);
}

#[test]
fn current_play_measure_index_jumps_to_ab_repeat_start_outside_active_range() {
    assert_eq!(current_play_measure_index(0, 4, Some((1, 2))), 1);
    assert_eq!(current_play_measure_index(2, 4, Some((1, 2))), 2);
}

#[test]
fn following_measure_index_wraps_inside_ab_repeat_range() {
    assert_eq!(following_measure_index(1, 4, Some((1, 2))), 2);
    assert_eq!(following_measure_index(2, 4, Some((1, 2))), 1);
}

#[test]
fn measure_indices_return_zero_when_effective_count_is_zero() {
    assert_eq!(current_play_measure_index(3, 0, None), 0);
    assert_eq!(following_measure_index(3, 0, None), 0);
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
