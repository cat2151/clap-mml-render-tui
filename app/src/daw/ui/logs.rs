use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::DawApp;

const LOG_BLOCK_DECORATION_HEIGHT: u16 = 2;
const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);

fn log_line_style(line: &str) -> Style {
    if line.starts_with("play: queue ") {
        Style::default().fg(MONOKAI_PINK)
    } else if line.starts_with("play: ") {
        Style::default().fg(Color::Yellow)
    } else if line.starts_with("cache: rerender done ") {
        Style::default().fg(Color::Green)
    } else if line.starts_with("cache: ") {
        Style::default().fg(Color::Cyan)
    } else if line.contains("error") || line.contains('✗') {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::White)
    }
}

pub(super) fn draw_logs(app: &DawApp, f: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let visible_height = area.height.saturating_sub(LOG_BLOCK_DECORATION_HEIGHT) as usize;
    let mut visible_lines: Vec<Line> = {
        let log_lines = app.log_lines.lock().unwrap();
        log_lines
            .iter()
            .rev()
            .take(visible_height)
            .cloned()
            .map(|line| Line::from(Span::styled(line.clone(), log_line_style(&line))))
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
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::White)),
        area,
    );
}
