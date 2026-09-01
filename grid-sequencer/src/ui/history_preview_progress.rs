//! Grid Historyで自動開始したpreview renderの進捗overlay。

use std::time::Duration;

use ratatui::{
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN},
    ui::centered_text_block_rect,
};

const TITLE: &str = " History Preview Rendering ";
const BAR_WIDTH: usize = 24;

pub(super) fn draw_overlay(
    frame: &mut Frame<'_>,
    completed: usize,
    total: usize,
    elapsed: Duration,
) {
    let lines = progress_lines(completed, total, elapsed);
    let area = centered_text_block_rect(frame.area(), TITLE, &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn progress_lines(completed: usize, total: usize, elapsed: Duration) -> Vec<Line<'static>> {
    let filled = if total == 0 {
        0
    } else {
        completed.min(total) * BAR_WIDTH / total
    };
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(BAR_WIDTH - filled));
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  render {completed}/{total} tracks  "),
                base_style().fg(MONOKAI_FG),
            ),
            Span::styled(bar, base_style().fg(MONOKAI_GREEN)),
        ]),
        Line::from(Span::styled(
            format!("  elapsed {}s", elapsed.as_secs()),
            base_style().fg(MONOKAI_CYAN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Space: stop  Esc/q/Shift+H: Historyを閉じる",
            base_style().fg(MONOKAI_GRAY),
        )),
    ]
}

#[cfg(test)]
mod tests;
