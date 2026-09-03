use super::{live_instance_for_grid_row, MAX_LIVE_TRACKS};
use crate::tracks::{CHORD_TRACK, FIRST_PLAYABLE_TRACK, TEMPO_TRACK};

#[test]
fn the_first_playable_row_takes_instance_zero_and_the_rest_follow_by_one() {
    assert_eq!(live_instance_for_grid_row(FIRST_PLAYABLE_TRACK), Some(0));
    assert_eq!(live_instance_for_grid_row(2), Some(0));
    assert_eq!(live_instance_for_grid_row(3), Some(1));
    assert_eq!(live_instance_for_grid_row(17), Some(15));
}

#[test]
fn rows_that_never_make_sound_have_no_instance() {
    assert_eq!(live_instance_for_grid_row(TEMPO_TRACK), None);
    assert_eq!(live_instance_for_grid_row(CHORD_TRACK), None);
}

#[test]
fn rows_beyond_the_server_instance_limit_have_no_instance() {
    let last = FIRST_PLAYABLE_TRACK + MAX_LIVE_TRACKS - 1;

    assert!(live_instance_for_grid_row(last).is_some());
    assert_eq!(live_instance_for_grid_row(last + 1), None);
    assert_eq!(live_instance_for_grid_row(18), None);
    assert_eq!(live_instance_for_grid_row(usize::MAX), None);
}

#[test]
fn every_instance_id_is_used_exactly_once_and_stays_within_the_limit() {
    let ids: Vec<_> = (FIRST_PLAYABLE_TRACK..FIRST_PLAYABLE_TRACK + MAX_LIVE_TRACKS)
        .map(|row| live_instance_for_grid_row(row).expect("within the limit"))
        .collect();

    assert_eq!(ids, (0..MAX_LIVE_TRACKS as u8).collect::<Vec<_>>());
}
