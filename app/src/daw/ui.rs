//! DAW モードの描画

mod grid;
mod help;
mod logs;
mod status;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};

use super::{CacheState, DawApp, DawMode};

/// Pending インジケータのアニメーション 1 フレームの長さ（ミリ秒）
const ANIM_FRAME_MS: u128 = 250;
/// Pending インジケータのアニメーションフレーム数（"." / ".." / "..."）
const ANIM_FRAME_COUNT: u128 = 3;
pub(super) const MONOKAI_BG: Color = Color::Rgb(39, 40, 34);
pub(super) const MONOKAI_FG: Color = Color::Rgb(248, 248, 242);
pub(super) const MONOKAI_GRAY: Color = Color::Rgb(160, 160, 160);
pub(super) const MONOKAI_PINK: Color = Color::Rgb(249, 38, 114);
pub(super) const MONOKAI_YELLOW: Color = Color::Rgb(230, 219, 116);
pub(super) const MONOKAI_GREEN: Color = Color::Rgb(166, 226, 46);
pub(super) const MONOKAI_CYAN: Color = Color::Rgb(102, 217, 239);
pub(super) const MONOKAI_PURPLE: Color = Color::Rgb(174, 129, 255);

fn cache_text_color(cs: &CacheState) -> Color {
    match cs {
        CacheState::Empty => MONOKAI_GRAY,
        CacheState::Pending | CacheState::Rendering | CacheState::Ready => MONOKAI_FG,
        CacheState::Error => Color::Red,
    }
}

fn cache_indicator(cs: &CacheState, anim_frame: u128) -> &'static str {
    match cs {
        CacheState::Empty => "     ",
        CacheState::Pending => ".    ",
        CacheState::Rendering => match anim_frame {
            0 => ".    ",
            1 => "..   ",
            _ => "...  ",
        },
        CacheState::Ready => "     ",
        CacheState::Error => "✗    ",
    }
}

fn cache_indicator_color(cs: &CacheState) -> Color {
    match cs {
        CacheState::Empty | CacheState::Ready => MONOKAI_GRAY,
        CacheState::Pending | CacheState::Rendering => MONOKAI_FG,
        CacheState::Error => Color::Red,
    }
}

fn loop_status_label(mmls: &[String]) -> Option<String> {
    super::playback::effective_measure_count(mmls).map(|count| {
        if count == 1 {
            "loop: meas1のみ (1小節)".to_string()
        } else {
            format!("loop: meas1〜meas{count} ({count}小節)")
        }
    })
}

fn loop_measure_summary_label(mmls: &[String]) -> Option<String> {
    super::playback::loop_measure_summary_label(mmls)
}

pub(super) fn draw(app: &DawApp, f: &mut Frame) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(area);

    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    grid::draw_grid(app, f, body_chunks[0]);
    logs::draw_logs(app, f, body_chunks[1]);
    status::draw_status(app, f, chunks[1], chunks[2], chunks[3]);

    if app.mode == DawMode::Help {
        help::draw_help(f, inner);
    }
}

#[cfg(test)]
#[path = "../tests/daw/ui.rs"]
mod tests;
