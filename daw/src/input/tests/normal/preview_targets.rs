use super::*;

#[test]
fn preview_target_tracks_can_force_current_track_even_when_solo_mode_differs() {
    let target_tracks = preview_target_tracks(3, 2, false).expect("playable current track");

    assert_eq!(target_tracks, vec![2]);
}

#[test]
fn preview_target_tracks_can_temporarily_open_all_tracks() {
    let target_tracks = preview_target_tracks(3, 2, true).expect("all-track preview");

    assert_eq!(target_tracks, vec![1, 2]);
}

#[test]
fn preview_target_tracks_rejects_non_playable_current_track() {
    assert_eq!(preview_target_tracks(3, 0, false), None);
}
