use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::DawApp;
use super::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW};
use crate::logging::{SHIFT_SPACE_LOG_LINE, STARTUP_LOG_PREFIX, STARTUP_LOG_SUFFIX};

const LOG_BLOCK_DECORATION_HEIGHT: u16 = 2;

fn is_error_log(line: &str) -> bool {
    line.contains("error") || line.contains("failed") || line.contains('✗')
}

fn log_line_style(line: &str) -> Style {
    if is_error_log(line) {
        Style::default().fg(Color::Red)
    } else if line == SHIFT_SPACE_LOG_LINE {
        Style::default()
            .fg(MONOKAI_PINK)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("起動時自動演奏: 本演奏（Shift+Space相当）")
        || line.starts_with("play: queue ")
    {
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

fn log_display_line(line: String) -> Line<'static> {
    if let Some(timestamp) = line
        .strip_prefix(STARTUP_LOG_PREFIX)
        .and_then(|text| text.strip_suffix(STARTUP_LOG_SUFFIX))
        .and_then(|text| text.strip_suffix(" JST"))
    {
        if let Some((date, time)) = timestamp.split_once(' ') {
            return Line::from(vec![
                Span::styled(
                    format!("{STARTUP_LOG_PREFIX}{date} "),
                    Style::default().fg(MONOKAI_FG),
                ),
                Span::styled(time.to_owned(), Style::default().fg(MONOKAI_PINK)),
                Span::styled(
                    format!(" JST{STARTUP_LOG_SUFFIX}"),
                    Style::default().fg(MONOKAI_FG),
                ),
            ]);
        }
    }
    let style = log_line_style(&line);
    Line::from(Span::styled(line, style))
}

pub(super) fn draw_logs(app: &DawApp, f: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let visible_height = area.height.saturating_sub(LOG_BLOCK_DECORATION_HEIGHT) as usize;
    let recent_lines: Vec<String> = {
        let lock_wait = crate::performance_log::SlowOperation::new("draw-log-lines-lock");
        let log_lines = app.log_lines.lock().unwrap();
        drop(lock_wait);
        log_lines
            .iter()
            .rev()
            .take(visible_height)
            .cloned()
            .collect()
    };
    let mut visible_lines: Vec<Line> = recent_lines
        .into_iter()
        .rev()
        .map(log_display_line)
        .collect();

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

#[cfg(test)]
mod tests;
