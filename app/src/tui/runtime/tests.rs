use ratatui::{backend::TestBackend, widgets::Paragraph, Terminal};

use super::*;

fn draw_remnant(terminal: &mut Terminal<TestBackend>) {
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new("old pane"), frame.area()))
        .unwrap();
}

fn first_cell(terminal: &Terminal<TestBackend>) -> &str {
    terminal.backend().buffer().cell((0, 0)).unwrap().symbol()
}

#[test]
fn terminal_is_cleared_on_first_draw_and_primary_screen_changes_only() {
    let mut terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
    let mut rendered_screen = None;

    draw_remnant(&mut terminal);
    clear_terminal_for_new_screen(
        &mut terminal,
        &mut rendered_screen,
        PrimaryScreen::LoopBrowser,
    )
    .unwrap();
    assert_eq!(first_cell(&terminal), " ");

    draw_remnant(&mut terminal);
    clear_terminal_for_new_screen(
        &mut terminal,
        &mut rendered_screen,
        PrimaryScreen::LoopBrowser,
    )
    .unwrap();
    assert_eq!(
        first_cell(&terminal),
        "o",
        "同一画面では毎frame clearしない"
    );

    clear_terminal_for_new_screen(
        &mut terminal,
        &mut rendered_screen,
        PrimaryScreen::GridSequencer,
    )
    .unwrap();
    assert_eq!(first_cell(&terminal), " ");
}
