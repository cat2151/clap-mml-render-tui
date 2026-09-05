use super::{
    cell_width, column_width, column_x_offset, fit_cell_text, measure_cell_width,
    MAX_MEASURE_CELL_WIDTH, MIN_MEASURE_CELL_WIDTH, TRACK_LABEL_WIDTH,
};

#[test]
fn init_column_is_wider_than_the_measure_columns() {
    assert_eq!(cell_width(0, MAX_MEASURE_CELL_WIDTH), 13);
    assert_eq!(column_width(0, MAX_MEASURE_CELL_WIDTH), 14);
}

#[test]
fn measure_columns_use_the_calculated_width() {
    for measure_index in 1..=8 {
        assert_eq!(cell_width(measure_index, 6), 6);
        assert_eq!(column_width(measure_index, 6), 7);
    }
}

#[test]
fn column_x_offset_accumulates_the_track_label_and_the_preceding_columns() {
    assert_eq!(column_x_offset(0, 6), TRACK_LABEL_WIDTH as u16);
    assert_eq!(column_x_offset(1, 6), 5 + 14);
    assert_eq!(column_x_offset(2, 6), 5 + 14 + 7);
    assert_eq!(column_x_offset(8, 6), 5 + 14 + 7 * 7);
}

#[test]
fn measure_width_uses_available_terminal_space_without_hiding_the_eighth_measure() {
    assert_eq!(measure_cell_width(58, 8), MIN_MEASURE_CELL_WIDTH);
    assert_eq!(measure_cell_width(78, 8), 6);
    assert_eq!(measure_cell_width(91, 8), MAX_MEASURE_CELL_WIDTH);
    assert_eq!(measure_cell_width(118, 8), MAX_MEASURE_CELL_WIDTH);
}

#[test]
fn truncated_cells_always_show_an_ellipsis() {
    assert_eq!(fit_cell_text("IIm7(b5)", 4), "IIm…");
    assert_eq!(fit_cell_text("IIm7(b5)", 6), "IIm7(…");
    assert_eq!(fit_cell_text("IIm7(b5)", 8), "IIm7(b5)");
}
