use std::time::{Duration, Instant};

use ratatui::{backend::TestBackend, Terminal};

use super::*;
use crate::GridSequencerScreen;

fn rendered_grid(
    screen: &GridSequencerScreen,
    title: &str,
    display: &[[Option<u8>; GRID_STEPS]],
) -> String {
    let mut terminal = Terminal::new(TestBackend::new(90, 6)).unwrap();
    let connection = screen.connection_status();
    terminal
        .draw(|frame| {
            draw(
                frame,
                frame.area(),
                title.to_string(),
                display,
                screen.state.step_index(),
                &connection,
                &[0],
            )
        })
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

fn sounding_screen() -> GridSequencerScreen {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].pattern.draw_span(0, 0);
    screen.state.rows_mut()[0].pattern.draw_span(4, 4);
    screen.state.start(now);
    screen.state.poll_steps(now, Duration::ZERO);
    screen
}

/// CC1 は全stepで送るので、鳴らないステップにも値が出る（無音セルの `.` は出ない）。
#[test]
fn the_cc1_grid_shows_a_value_on_every_step() {
    let screen = sounding_screen();

    let rendered = rendered_grid(&screen, " CC1 Modulation ", screen.state.cc1_display());

    assert!(rendered.contains("CC1 Modulation"), "{rendered}");
    assert!(
        rendered.contains("127") || rendered.contains('0'),
        "{rendered}"
    );
    assert!(!rendered.contains('.'), "{rendered}");
}

#[test]
fn the_velocity_grid_shows_values_only_on_note_trigger_steps() {
    let screen = sounding_screen();

    let rendered = rendered_grid(&screen, " Velocity ", screen.state.velocity_display());

    assert!(rendered.contains("Velocity"), "{rendered}");
    assert!(
        rendered.contains("100") || rendered.contains("127"),
        "{rendered}"
    );
    assert!(rendered.contains('.'), "{rendered}");
}

#[test]
fn cells_use_four_columns_so_three_digit_values_stay_aligned() {
    assert_eq!(cell_text(Some(0)), "0   ");
    assert_eq!(cell_text(Some(100)), "100 ");
    assert_eq!(cell_text(Some(127)), "127 ");
    assert_eq!(cell_text(None), ".   ");
    assert_eq!(step_ruler().chars().count(), GRID_STEPS * CELL_WIDTH);
}
