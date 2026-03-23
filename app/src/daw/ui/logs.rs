use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::DawApp;

pub(super) fn draw_logs(app: &DawApp, f: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let visible_height = area.height.saturating_sub(1) as usize;
    let mut visible_lines: Vec<Line> = {
        let log_lines = app.log_lines.lock().unwrap();
        log_lines
            .iter()
            .rev()
            .take(visible_height)
            .cloned()
            .map(Line::from)
            .collect()
    };
    visible_lines.reverse();

    if visible_lines.is_empty() && visible_height > 0 {
        visible_lines.push(Line::from("(no log)"));
    }

    f.render_widget(
        Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .title(" log ")
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::White)),
        area,
    );
}
