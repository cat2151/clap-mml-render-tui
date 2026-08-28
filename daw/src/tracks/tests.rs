use super::*;

#[test]
fn saved_track_numbers_skip_the_chord_row() {
    assert_eq!(grid_row_from_saved_track(0), TEMPO_TRACK);
    assert_eq!(grid_row_from_saved_track(1), FIRST_PLAYABLE_TRACK);
    assert_eq!(grid_row_from_saved_track(8), 9);

    assert_eq!(saved_track_from_grid_row(TEMPO_TRACK), Some(0));
    assert_eq!(saved_track_from_grid_row(CHORD_TRACK), None);
    assert_eq!(saved_track_from_grid_row(FIRST_PLAYABLE_TRACK), Some(1));
    assert_eq!(saved_track_from_grid_row(9), Some(8));
}

#[test]
fn the_saved_and_grid_track_counts_differ_by_the_chord_row() {
    assert_eq!(grid_track_count_from_saved(9), crate::TRACKS);
    assert_eq!(saved_track_count_from_grid(crate::TRACKS), 9);
}

#[test]
fn only_the_chord_row_is_excluded_from_rendering() {
    assert!(track_renders_audio(TEMPO_TRACK));
    assert!(!track_renders_audio(CHORD_TRACK));
    assert!(track_renders_audio(FIRST_PLAYABLE_TRACK));
}

#[test]
fn the_track_label_keeps_the_saved_track_number() {
    assert_eq!(track_label(TEMPO_TRACK), "Tempo");
    assert_eq!(track_label(CHORD_TRACK), "Chord");
    assert_eq!(track_label(FIRST_PLAYABLE_TRACK), "T1");
    assert_eq!(track_label(9), "T8");
}
