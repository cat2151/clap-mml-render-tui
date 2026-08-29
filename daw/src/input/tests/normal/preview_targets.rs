use super::*;

#[test]
fn preview_target_tracks_can_force_current_track_even_when_solo_mode_differs() {
    let target_tracks = preview_target_tracks(4, 3, false).expect("playable current track");

    assert_eq!(target_tracks, vec![3]);
}

#[test]
fn preview_target_tracks_can_temporarily_open_all_tracks() {
    let target_tracks = preview_target_tracks(4, 3, true).expect("all-track preview");

    assert_eq!(target_tracks, vec![2, 3]);
}

#[test]
fn preview_target_tracks_rejects_non_playable_current_track() {
    assert_eq!(preview_target_tracks(4, crate::CHORD_TRACK, false), None);
}

#[test]
fn cursor_move_preview_uses_the_first_track_generated_from_the_chord_row() {
    let mut data = vec![vec![String::new(); 3]; 5];
    data[3][0] = r#"{"generate from chord track":"drop2"}"#.to_string();
    data[4][0] = r#"{"generate from chord track":"close"}"#.to_string();

    assert_eq!(
        cursor_move_preview_track(&data, data.len(), crate::CHORD_TRACK),
        Some(3)
    );
}

#[test]
fn cursor_move_preview_rejects_a_chord_row_without_a_generated_track() {
    let data = vec![vec![String::new(); 3]; 4];

    assert_eq!(
        cursor_move_preview_track(&data, data.len(), crate::CHORD_TRACK),
        None
    );
}
