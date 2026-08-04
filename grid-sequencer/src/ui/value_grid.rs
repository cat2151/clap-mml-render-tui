//! 小節頭で抽選した値の grid。CC1 modulation と velocity で共用する。

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_DARK_GRAY, MONOKAI_GRAY, MONOKAI_GREEN},
};

use crate::{GridConnectionStatus, GridRowReadiness, GRID_STEPS};

const CELL_WIDTH: usize = 4;

/// `accent` の値を持つセルだけを目立たせて描く。
pub(super) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    display: &[[Option<u8>; GRID_STEPS]],
    playhead: usize,
    connection: &GridConnectionStatus,
    accent: u8,
) {
    let mut lines = vec![header_line()];
    lines.extend(display.iter().enumerate().map(|(row, values)| {
        row_line(row, values, playhead, connection.row_readiness(row), accent)
    }));
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn header_line() -> Line<'static> {
    let style = base_style().fg(MONOKAI_GRAY);
    Line::from(vec![
        Span::styled("  # ", style),
        Span::styled(step_ruler(), style),
    ])
}

fn row_line(
    row: usize,
    values: &[Option<u8>; GRID_STEPS],
    playhead: usize,
    readiness: GridRowReadiness,
    accent: u8,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!(" {:>2} ", row + 1),
        label_style(readiness),
    )];
    for (step, value) in values.iter().enumerate() {
        let style = cell_style(readiness, *value, accent);
        let style = if step == playhead {
            cursor_highlight_style(style)
        } else {
            style
        };
        spans.push(Span::styled(cell_text(*value), style));
    }
    Line::from(spans)
}

fn cell_text(value: Option<u8>) -> String {
    match value {
        Some(value) => format!("{value:<CELL_WIDTH$}"),
        None => format!("{:<CELL_WIDTH$}", "."),
    }
}

fn label_style(readiness: GridRowReadiness) -> Style {
    match readiness {
        GridRowReadiness::Prepared => base_style(),
        GridRowReadiness::InstanceReady => base_style().fg(MONOKAI_GRAY),
        GridRowReadiness::Pending => base_style().fg(MONOKAI_DARK_GRAY),
    }
}

fn cell_style(readiness: GridRowReadiness, value: Option<u8>, accent: u8) -> Style {
    let color = match readiness {
        GridRowReadiness::Prepared if value == Some(accent) => MONOKAI_GREEN,
        GridRowReadiness::Prepared | GridRowReadiness::InstanceReady => MONOKAI_GRAY,
        GridRowReadiness::Pending => MONOKAI_DARK_GRAY,
    };
    base_style().fg(color)
}

fn step_ruler() -> String {
    (0..GRID_STEPS)
        .map(|step| {
            if step % 4 == 0 {
                format!("{:<CELL_WIDTH$}", step + 1)
            } else {
                " ".repeat(CELL_WIDTH)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
