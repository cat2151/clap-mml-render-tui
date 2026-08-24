//! NOTE の最終 track 直下に置く、auto random patch 先読みの進捗欄。

use std::time::Duration;

use ratatui::{layout::Rect, style::Color, widgets::Paragraph, Frame};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW},
};

use crate::GridConnectionStatus;

pub(super) fn draw(f: &mut Frame<'_>, area: Rect, connection: &GridConnectionStatus) {
    if area.height == 0 {
        return;
    }
    let (text, color) = progress_text(connection, area.width);
    f.render_widget(Paragraph::new(text).style(base_style().fg(color)), area);
}

fn progress_text(connection: &GridConnectionStatus, width: u16) -> (String, Color) {
    let progress = connection.preload;
    if progress.total == 0 {
        let text = if width >= 60 {
            " AUTO PATCH LOAD | idle "
        } else {
            " PATCH LOAD | idle "
        };
        return (text.to_string(), MONOKAI_GRAY);
    }
    if progress.completed >= progress.total {
        if connection.preload_failed {
            let text = if width >= 60 {
                format!(
                    " AUTO PATCH LOAD | failed {}/{} | ETA 0.0s ",
                    progress.completed, progress.total
                )
            } else {
                format!(
                    " PATCH LOAD | {}/{} failed ",
                    progress.completed, progress.total
                )
            };
            return (text, MONOKAI_PINK);
        }
        let elapsed = format_eta(connection.preload_measured_elapsed().unwrap_or_default());
        let text = if width >= 60 {
            format!(
                " AUTO PATCH LOAD | complete {}/{} | load {elapsed} ",
                progress.completed, progress.total,
            )
        } else {
            format!(
                " PATCH LOAD | {}/{} done | {elapsed} ",
                progress.completed, progress.total,
            )
        };
        return (text, MONOKAI_GREEN);
    }
    let eta = connection
        .preload_eta()
        .map(format_eta)
        .unwrap_or_else(|| "--".to_string());
    let current = connection.preload_current_instance();
    let text = match (current, width >= 60) {
        (Some(current), true) => format!(
            " AUTO PATCH LOAD | loading instance {current}/{} | ETA {eta} ",
            progress.total
        ),
        (None, true) => format!(
            " AUTO PATCH LOAD | next instance {}/{} | ETA {eta} ",
            progress.completed + 1,
            progress.total
        ),
        (Some(current), false) => {
            format!(
                " PATCH LOAD | inst {current}/{} | ETA {eta} ",
                progress.total
            )
        }
        (None, false) => format!(
            " PATCH LOAD | next {}/{} | ETA {eta} ",
            progress.completed + 1,
            progress.total
        ),
    };
    (text, MONOKAI_YELLOW)
}

fn format_eta(duration: Duration) -> String {
    let tenths = duration.as_millis().saturating_add(50) / 100;
    if tenths < 600 {
        return format!("{}.{:01}s", tenths / 10, tenths % 10);
    }
    let seconds = tenths.saturating_add(5) / 10;
    format!("{}m {:02}s", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests;
