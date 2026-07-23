use super::*;

#[test]
fn volume_adjustment_uses_three_db_steps_and_shared_bounds() {
    let mut volume_db = 0;
    assert!(adjust_volume_db(&mut volume_db, -MIXER_STEP_DB));
    assert_eq!(volume_db, -3);

    volume_db = MIXER_MIN_DB;
    assert!(!adjust_volume_db(&mut volume_db, -MIXER_STEP_DB));
    volume_db = MIXER_MAX_DB;
    assert!(!adjust_volume_db(&mut volume_db, MIXER_STEP_DB));
}

#[test]
fn db_is_converted_to_linear_gain() {
    assert!((volume_db_to_gain(-6) - 10.0f32.powf(-6.0 / 20.0)).abs() < f32::EPSILON);
}
