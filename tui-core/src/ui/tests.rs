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

/// 端末が受け取る内容（TestBackend の buffer）で、overlay の左上の角が出るかを見る。
fn overlay_corner_over_wide_text(clear: fn(&mut Frame<'_>, Rect)) -> String {
    let mut terminal = Terminal::new(TestBackend::new(8, 2)).unwrap();
    terminal
        .draw(|f| {
            f.render_widget(Paragraph::new("aで"), f.area());
            let area = Rect::new(2, 0, 4, 2);
            clear(f, area);
            f.render_widget(Block::default().borders(Borders::ALL), area);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..4).map(|x| buffer[(x, 0)].symbol()).collect()
}

#[test]
fn clear_overlay_area_keeps_corner_when_wide_char_straddles_left_edge() {
    fn plain_clear(f: &mut Frame<'_>, area: Rect) {
        f.render_widget(Clear, area);
    }
    // 背面の「で」は x=1..2 を占め、overlay の左端 x=2 へはみ出す。
    assert_ne!(overlay_corner_over_wide_text(plain_clear), "a ┌─");
    assert_eq!(overlay_corner_over_wide_text(clear_overlay_area), "a ┌─");
}
