use super::*;

#[test]
fn underruns_raise_one_level_and_reset_the_level_counter() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 100);

    assert_eq!(
        buffer.observe(now + Duration::from_millis(50), 132),
        Some(4)
    );
    assert_eq!(buffer.multiplier(), 4);
    assert_eq!(buffer.underrun_frames(), 0);
    assert_eq!(
        buffer.observe(now + Duration::from_millis(100), 140),
        Some(8)
    );
    assert_eq!(
        buffer.observe(now + Duration::from_millis(150), 148),
        Some(16)
    );
}

#[test]
fn underruns_climb_past_sixteen_up_to_the_maximum() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 0);
    let mut underruns = 0;
    for expected in [4, 8, 16, 32, 64, 128, 256] {
        underruns += 1;
        assert_eq!(buffer.observe(now, underruns), Some(expected));
    }
    assert_eq!(buffer.multiplier(), MAX_BUFFER_MULTIPLIER);
}

#[test]
fn maximum_level_accumulates_underruns_without_growing() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 0);
    let mut underruns = 0;
    while buffer.multiplier() < MAX_BUFFER_MULTIPLIER {
        underruns += 1;
        assert!(buffer.observe(now, underruns).is_some());
    }

    assert_eq!(buffer.observe(now, underruns + 8), None);
    assert_eq!(buffer.multiplier(), MAX_BUFFER_MULTIPLIER);
    assert_eq!(buffer.underrun_frames(), 8);
}

#[test]
fn ten_stable_seconds_lower_exactly_one_level_at_a_time() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 0);
    assert_eq!(buffer.observe(now, 1), Some(4));
    assert_eq!(buffer.observe(now, 2), Some(8));
    assert_eq!(buffer.observe(now, 3), Some(16));

    assert_eq!(
        buffer.observe(now + STABLE_INTERVAL - Duration::from_millis(1), 3),
        None
    );
    assert_eq!(buffer.observe(now + STABLE_INTERVAL, 3), Some(8));
    assert_eq!(buffer.observe(now + STABLE_INTERVAL * 2, 3), Some(4));
    assert_eq!(buffer.observe(now + STABLE_INTERVAL * 3, 3), Some(2));
    assert_eq!(buffer.observe(now + STABLE_INTERVAL * 4, 3), None);
}

#[test]
fn a_new_underrun_restarts_the_stability_period() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 0);
    assert_eq!(buffer.observe(now, 1), Some(4));

    let almost_stable = now + STABLE_INTERVAL - Duration::from_millis(1);
    assert_eq!(buffer.observe(almost_stable, 2), Some(8));
    assert_eq!(buffer.observe(now + STABLE_INTERVAL, 2), None);
    assert_eq!(buffer.observe(almost_stable + STABLE_INTERVAL, 2), Some(4));
}

#[test]
fn reverting_restores_the_multiplier_the_server_last_accepted() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 0);
    assert_eq!(buffer.observe(now, 1), Some(4));

    buffer.revert(2);

    assert_eq!(buffer.multiplier(), 2);
}

#[test]
fn a_server_counter_reset_rebases_without_raising_the_buffer() {
    let now = Instant::now();
    let mut buffer = AdaptiveBuffer::new(now, 1_000);

    assert_eq!(buffer.observe(now + Duration::from_secs(1), 5), None);
    assert_eq!(buffer.multiplier(), 2);
    assert_eq!(buffer.underrun_frames(), 0);
    assert_eq!(buffer.observe(now + Duration::from_secs(2), 8), Some(4));
}
