use cmrt_tui_core::theme::MONOKAI_YELLOW;

use super::{mark_cells, render, rows_in, screen_with_mml};
use crate::ui::layout_for;
use crate::Take;

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
