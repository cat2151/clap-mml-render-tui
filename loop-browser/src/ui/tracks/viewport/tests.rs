use super::*;

#[test]
fn sixteen_measures_fit_in_one_screen_instead_of_scrolling_at_thirteen() {
    let cell_width = measure_cell_width(209, 16);

    assert_eq!(cell_width, 13);
    assert!(209 / cell_width >= 16);
}

#[test]
fn a_short_loop_expands_to_one_column_per_thirty_second_note() {
    assert_eq!(measure_cell_width(209, 2), MAX_CELL_WIDTH);
    assert_eq!(MAX_CELL_WIDTH, 32);
}

#[test]
fn many_measures_are_packed_down_to_the_minimum_before_scrolling() {
    assert_eq!(measure_cell_width(209, 24), 8);
    assert_eq!(measure_cell_width(209, 64), MIN_CELL_WIDTH);
}

#[test]
fn an_empty_grid_does_not_divide_by_zero() {
    assert_eq!(measure_cell_width(209, 0), MAX_CELL_WIDTH);
    assert_eq!(measure_cell_width(0, 0), MIN_CELL_WIDTH);
}

#[test]
fn narrow_cells_keep_the_measure_number_by_dropping_the_word() {
    assert_eq!(measure_label(15, 16), "measure 16");
    assert_eq!(measure_label(15, 10), "measure 16");
    assert_eq!(measure_label(15, 8), "M16");
    assert_eq!(fit(&measure_label(15, 8), 8), "M16     ");
}
