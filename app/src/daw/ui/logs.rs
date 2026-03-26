use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::DawApp;

const LOG_BLOCK_DECORATION_HEIGHT: u16 = 2;
use super::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW};

fn is_error_log(line: &str) -> bool {
    line.contains("error") || line.contains("failed") || line.contains('✗')
}

fn log_line_style(line: &str) -> Style {
    if is_error_log(line) {
        Style::default().fg(Color::Red)
    } else if line.starts_with("play: queue ") {
        Style::default().fg(MONOKAI_PINK)
    } else if line.starts_with("play: ") {
        Style::default().fg(MONOKAI_YELLOW)
    } else if line.starts_with("cache: rerender done ") {
        Style::default().fg(MONOKAI_GREEN)
    } else if line.starts_with("cache: ") {
        Style::default().fg(MONOKAI_CYAN)
    } else {
        Style::default().fg(MONOKAI_FG)
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
            .map(|line| {
                let style = log_line_style(&line);
                Line::from(Span::styled(line, style))
            })
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
                    .border_style(Style::default().fg(MONOKAI_CYAN))
                    .style(Style::default().bg(MONOKAI_BG)),
            )
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)),
        area,
    );
}
