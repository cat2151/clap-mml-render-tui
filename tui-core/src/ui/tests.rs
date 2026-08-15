use ratatui::{
    backend::TestBackend,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::Block,
    Terminal,
};

use super::*;

#[test]
fn centered_rect_with_size_returns_zero_sized_area_unchanged() {
    assert_eq!(
        centered_rect_with_size(10, 10, Rect::new(3, 4, 0, 5)),
        Rect::new(3, 4, 0, 5)
    );
    assert_eq!(
        centered_rect_with_size(10, 10, Rect::new(3, 4, 5, 0)),
        Rect::new(3, 4, 5, 0)
    );
}

#[test]
fn centered_text_block_rect_clamps_large_content_to_area() {
    let area = Rect::new(10, 20, 40, 5);
    let lines = [Line::from("x".repeat(70_000))];

    let rect = centered_text_block_rect(area, " title ", &lines);

    assert_eq!(rect.width, area.width);
    assert_eq!(rect.height, 3);
}

#[test]
fn frame_background_replaces_every_previous_cell() {
    let mut terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Block::default().style(Style::default().bg(Color::Red)),
                frame.area(),
            );
        })
        .unwrap();

    terminal.draw(draw_frame_background).unwrap();

    assert!(terminal
        .backend()
        .buffer()
        .content
        .iter()
        .all(|cell| cell.bg == crate::theme::MONOKAI_BG));
}
