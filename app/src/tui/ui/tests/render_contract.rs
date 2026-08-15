use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Style},
    widgets::Block,
    Terminal,
};

use super::*;
use crate::screen_switch::PrimaryScreen;

fn text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_no_stale_red_cells(terminal: &Terminal<TestBackend>) {
    let stale = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| (cell.bg == Color::Red).then_some(index))
        .collect::<Vec<_>>();
    assert!(stale.is_empty(), "stale red cell indexes: {stale:?}");
}

#[test]
fn root_draw_replaces_cells_left_by_an_unknown_previous_frame() {
    let mut app = TuiApp::new_for_test(test_config());
    // ASCII中心の画面を使い、全角glyphの後続セル（TestBackendでは直前styleを保持する）を除外する。
    app.active_screen = PrimaryScreen::GridSequencer;
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Block::default().style(Style::default().bg(Color::Red)),
                frame.area(),
            );
        })
        .unwrap();

    terminal.draw(|frame| draw(&mut app, frame)).unwrap();

    assert_no_stale_red_cells(&terminal);
}

#[test]
fn patch_overlay_close_and_notepad_to_grid_redraw_leave_no_old_labels() {
    let mut app = TuiApp::new_for_test(test_config());
    let mut terminal = Terminal::new(TestBackend::new(90, 24)).unwrap();

    app.notepad.mode = Mode::PatchSelect;
    terminal.draw(|frame| draw(&mut app, frame)).unwrap();
    assert!(text(terminal.backend().buffer()).contains("Patches query"));

    app.notepad.mode = Mode::Normal;
    terminal.draw(|frame| draw(&mut app, frame)).unwrap();
    let normal = text(terminal.backend().buffer());
    assert!(!normal.contains("Patches query"));
    assert!(normal.contains("notepad mode"));

    app.active_screen = PrimaryScreen::GridSequencer;
    terminal.draw(|frame| draw(&mut app, frame)).unwrap();
    let grid = text(terminal.backend().buffer());
    assert!(!grid.contains("notepad mode"));
    assert!(grid.contains("Velocity"));
}
