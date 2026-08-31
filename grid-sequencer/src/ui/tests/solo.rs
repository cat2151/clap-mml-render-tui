use cmrt_tui_core::theme::{cursor_highlight_bg, MONOKAI_GRAY, MONOKAI_GREEN};

use super::*;

fn solo_screen() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::with_track_count(None, 3);
    for row in screen.state.rows_mut() {
        row.pattern.draw_span(0, 0);
    }
    screen.toggle_track_solo(0);
    screen
}

#[test]
fn note_grid_shows_s_for_solo_and_lowercase_m_for_derived_mute() {
    let screen = solo_screen();
    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();
    let solo_column = usize::from(test_layout(&screen).solo_column());

    assert_eq!(slice_chars(lines[FIRST_ROW_Y], solo_column, 1), "S");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y + 1], solo_column, 1), "m");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y + 2], solo_column, 1), "m");
}

#[test]
fn derived_muted_note_cc1_and_velocity_rows_are_grey() {
    let screen = solo_screen();
    let connection = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        ..GridConnectionStatus::default()
    };
    let terminal = terminal_with_connection(&screen, &connection);
    let buffer = terminal.backend().buffer();
    let layout = test_layout(&screen);

    let note_first_cell = layout.step_column(0);
    assert_eq!(
        buffer
            .cell((note_first_cell, layout.note.y + 2))
            .unwrap()
            .fg,
        MONOKAI_GREEN
    );
    assert_eq!(
        buffer
            .cell((note_first_cell, layout.note.y + 3))
            .unwrap()
            .fg,
        MONOKAI_GRAY
    );

    let value_first_cell = layout.cc1.x + 5;
    assert_eq!(
        buffer
            .cell((value_first_cell, layout.cc1.y + 3))
            .unwrap()
            .fg,
        MONOKAI_GRAY
    );
    assert_eq!(
        buffer
            .cell((layout.velocity.x + 5, layout.velocity.y + 3))
            .unwrap()
            .fg,
        MONOKAI_GRAY
    );
}

#[test]
fn only_the_selected_tracks_number_has_the_cursor_background() {
    let mut screen = solo_screen();
    screen.select_track(1);
    let terminal = terminal_for(&screen);
    let buffer = terminal.backend().buffer();
    let number_x = test_layout(&screen).note.x + 2;
    let first = buffer.cell((number_x, FIRST_ROW_Y as u16)).unwrap();
    let second = buffer.cell((number_x, (FIRST_ROW_Y + 1) as u16)).unwrap();

    assert_ne!(first.bg, cursor_highlight_bg(first.fg));
    assert_eq!(second.bg, cursor_highlight_bg(second.fg));
}
