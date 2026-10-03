use cmrt_tui_core::theme::MONOKAI_YELLOW;

use ratatui::buffer::Buffer;

use super::long_material::{long_mml_screen, long_smf_screen, render_playing};
use super::{mark_cells, render, rows_in, screen_with_mml};
use crate::ui::layout_for;
use crate::{GuitarArticulationScreen, Take};

#[test]
fn the_playhead_row_sits_on_top_and_marks_the_column_being_played() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.set_playhead(Some((Take::Converted, 0.3)));
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;

    assert!(rows_in(&buffer, matrix)[1].starts_with("│play"));
    let playhead = mark_cells(&buffer, matrix, "play");
    assert_eq!(playhead.len(), 1);
    let (x, y) = playhead[0];
    let cell = buffer.cell((x, y)).unwrap();
    assert_eq!((cell.symbol(), cell.fg), ("▼", MONOKAI_YELLOW));
    let note = mark_cells(&buffer, matrix, "F#2")
        .into_iter()
        .find(|&cell| buffer.cell(cell).unwrap().symbol() == "■")
        .unwrap();
    assert_eq!(x, note.0);
}

#[test]
fn the_playhead_row_stays_empty_while_nothing_plays() {
    let screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;

    assert!(mark_cells(&buffer, matrix, "play").is_empty());
}

#[test]
fn the_matrix_scrolls_to_keep_the_playhead_in_sight() {
    let mml = format!("o3 l16 {}", "cdefgab".repeat(10));
    let mut screen = screen_with_mml(&mml);
    let last = screen.column_count() - 1;
    let last_on = screen.column_on_seconds(last, Take::Plain).unwrap();
    screen.set_playhead(Some((Take::Plain, last_on)));
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;

    assert_eq!(screen.playhead_column(), Some(last));
    assert_eq!(mark_cells(&buffer, matrix, "play").len(), 1);
}

/// 見えている列の数と、`▼` が左から何列目か。列は音高の段 `pitch_label` の升（■ か ·）で数える。
fn visible_and_playhead(
    buffer: &Buffer,
    screen: &GuitarArticulationScreen,
    pitch_label: &str,
) -> (usize, usize) {
    let matrix = layout_for(buffer.area, screen).matrix;
    let columns: Vec<u16> = mark_cells(buffer, matrix, pitch_label)
        .into_iter()
        .map(|(x, _)| x)
        .collect();
    let playhead = mark_cells(buffer, matrix, "play");
    assert_eq!(playhead.len(), 1);
    let index = columns
        .iter()
        .position(|&x| x == playhead[0].0)
        .expect("▼ は見えている列の上に在る");
    (columns.len(), index)
}

fn assert_centered(visible: usize, index: usize) {
    let left = index;
    let right = visible - 1 - index;
    assert!(left.abs_diff(right) <= 1, "左 {left} 列・右 {right} 列");
}

#[test]
fn the_playhead_in_the_middle_of_a_long_line_sits_at_the_center() {
    let mut screen = long_mml_screen();
    let buffer = render_playing(&mut screen, Take::Plain, 70);

    let (visible, index) = visible_and_playhead(&buffer, &screen, "C2");
    assert!(visible < screen.column_count());
    assert_centered(visible, index);
}

#[test]
fn near_the_start_the_matrix_does_not_scroll() {
    let mut screen = long_mml_screen();
    let buffer = render_playing(&mut screen, Take::Plain, 3);

    let (_, index) = visible_and_playhead(&buffer, &screen, "C2");
    assert_eq!(index, 3);
}

#[test]
fn at_the_last_column_the_matrix_stops_with_the_last_column_at_the_right_edge() {
    let mut screen = long_mml_screen();
    let middle = render_playing(&mut screen, Take::Plain, 70);
    let (fit, _) = visible_and_playhead(&middle, &screen, "C2");
    let last = screen.column_count() - 1;
    let buffer = render_playing(&mut screen, Take::Plain, last);

    let (visible, index) = visible_and_playhead(&buffer, &screen, "C2");
    assert_eq!(visible, fit);
    assert_eq!(index, visible - 1);
}

#[test]
fn an_smf_material_also_keeps_the_playhead_at_the_center() {
    let mut screen = long_smf_screen(300);
    let buffer = render_playing(&mut screen, Take::Converted, 150);

    let (visible, index) = visible_and_playhead(&buffer, &screen, "C4");
    assert!(visible < screen.column_count());
    assert_centered(visible, index);
}
