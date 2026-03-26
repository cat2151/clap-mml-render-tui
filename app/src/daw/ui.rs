//! DAW モードの描画

mod grid;
mod help;
mod logs;
mod status;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    Frame,
};

use super::{CacheState, DawApp, DawMode};

/// Pending インジケータのアニメーション 1 フレームの長さ（ミリ秒）
const ANIM_FRAME_MS: u128 = 250;
/// Pending インジケータのアニメーションフレーム数（"." / ".." / "..."）
const ANIM_FRAME_COUNT: u128 = 3;

fn cache_text_color(cs: &CacheState) -> Color {
    match cs {
        CacheState::Empty => Color::DarkGray,
        CacheState::Pending | CacheState::Rendering | CacheState::Ready => Color::White,
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
        CacheState::Empty | CacheState::Ready => Color::DarkGray,
        CacheState::Pending | CacheState::Rendering => Color::White,
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    grid::draw_grid(app, f, body_chunks[0]);
    logs::draw_logs(app, f, body_chunks[1]);
    status::draw_status(app, f, chunks[1], chunks[2], chunks[3]);

    if app.mode == DawMode::Help {
        help::draw_help(f, area);
    }
}

#[cfg(test)]
#[path = "../tests/daw/ui.rs"]
mod tests;
