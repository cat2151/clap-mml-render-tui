use super::{cell_width, column_width, column_x_offset, TRACK_LABEL_WIDTH};

#[test]
fn init_column_is_wider_than_the_measure_columns() {
    assert_eq!(cell_width(0), 13);
    assert_eq!(column_width(0), 14);
}

#[test]
fn measure_columns_keep_the_original_five_digit_pitch() {
    for measure_index in 1..=8 {
        assert_eq!(cell_width(measure_index), 4);
        assert_eq!(column_width(measure_index), 5);
    }
}

#[test]
fn column_x_offset_accumulates_the_track_label_and_the_preceding_columns() {
    assert_eq!(column_x_offset(0), TRACK_LABEL_WIDTH as u16);
    assert_eq!(column_x_offset(1), 5 + 14);
    assert_eq!(column_x_offset(2), 5 + 14 + 5);
    assert_eq!(column_x_offset(8), 5 + 14 + 5 * 7);
}
