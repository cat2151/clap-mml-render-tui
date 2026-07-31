use super::*;

#[test]
fn status_starts_idle_with_no_gain_reduction() {
    let status = GridConnectionStatus::default();
    assert_eq!(status.phase, GridConnectionPhase::Idle);
    assert_eq!(status.limiter_reduction_db, 0.0);
}

#[test]
fn successful_send_updates_gain_reduction() {
    let mut status = GridConnectionStatus::default();
    status.apply_result(
        Ok(LimiterMeter {
            current_reduction_db: 1.0,
            peak_reduction_db: 3.5,
        }),
        None,
        false,
    );
    assert_eq!(status.phase, GridConnectionPhase::Ready);
    assert_eq!(status.limiter_reduction_db, 3.5);
}

#[test]
fn boosted_rows_are_named_with_one_based_numbers() {
    assert_eq!(describe_boosted(&[0.0, 0.0, 0.0]), "none");
    assert_eq!(describe_boosted(&[6.0, 0.0, 0.0]), "row1:+6dB");
    assert_eq!(describe_boosted(&[0.0, -6.0, 6.0]), "row2:-6dB,row3:+6dB");
}
