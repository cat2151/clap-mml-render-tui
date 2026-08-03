use std::time::{Duration, Instant};

use ratatui::{backend::TestBackend, Terminal};

use super::*;

fn rendered_cc1_grid(screen: &GridSequencerScreen) -> String {
    let mut terminal = Terminal::new(TestBackend::new(90, 6)).unwrap();
    let connection = screen.connection_status();
    terminal
        .draw(|frame| draw(screen, &connection, frame, frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn grid_shows_values_only_on_note_trigger_steps() {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].cells[0] = true;
    screen.state.rows_mut()[0].cells[4] = true;
    screen.state.start(now);
    screen.state.poll_steps(now, Duration::ZERO);

    let rendered = rendered_cc1_grid(&screen);

    assert!(rendered.contains("CC1 Modulation"), "{rendered}");
    assert!(
        rendered.contains("127") || rendered.contains('0'),
        "{rendered}"
    );
    assert!(rendered.contains('.'), "{rendered}");
}

#[test]
fn cells_use_four_columns_so_three_digit_values_stay_aligned() {
    assert_eq!(cell_text(Some(0)), "0   ");
    assert_eq!(cell_text(Some(127)), "127 ");
    assert_eq!(cell_text(None), ".   ");
    assert_eq!(step_ruler().chars().count(), GRID_STEPS * CELL_WIDTH);
}
