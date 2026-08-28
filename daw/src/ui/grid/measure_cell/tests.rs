use super::generated_cell_text;

const GENERATED_TRACK: usize = crate::FIRST_PLAYABLE_TRACK;
const PLAIN_TRACK: usize = crate::FIRST_PLAYABLE_TRACK + 1;

/// 0 = Tempo / 1 = chord 行 / 2 = 生成対象 track / 3 = 生成しない track。
fn data_with_chord_row() -> Vec<Vec<String>> {
    let mut data = vec![vec![String::new(); 4]; crate::FIRST_PLAYABLE_TRACK + 2];
    data[crate::CHORD_TRACK][1] = "I-IV".to_string();
    data[GENERATED_TRACK][0] = r#"{"generate from chord track": "close"}"#.to_string();
    data
}

#[test]
fn a_generated_cell_borrows_the_chord_row_cell() {
    let data = data_with_chord_row();

    assert_eq!(
        generated_cell_text(&data, GENERATED_TRACK, 1).as_deref(),
        Some("I-IV")
    );
}

#[test]
fn a_track_without_the_generate_key_shows_nothing() {
    let data = data_with_chord_row();

    assert_eq!(generated_cell_text(&data, PLAIN_TRACK, 1), None);
}

#[test]
fn a_handwritten_cell_is_left_to_the_caller() {
    let mut data = data_with_chord_row();
    data[GENERATED_TRACK][1] = "cde".to_string();

    assert_eq!(generated_cell_text(&data, GENERATED_TRACK, 1), None);
}

#[test]
fn a_measure_whose_chord_row_cell_is_empty_shows_nothing() {
    let data = data_with_chord_row();

    // chord 行の meas2 は空なので、生成対象 track の meas2 も鳴らないし何も出ない。
    assert_eq!(generated_cell_text(&data, GENERATED_TRACK, 2), None);
}

#[test]
fn the_chord_row_itself_never_borrows_from_itself() {
    let data = data_with_chord_row();

    assert_eq!(generated_cell_text(&data, crate::CHORD_TRACK, 1), None);
}

#[test]
fn the_init_column_is_not_a_generated_cell() {
    let data = data_with_chord_row();

    assert_eq!(generated_cell_text(&data, GENERATED_TRACK, 0), None);
}
