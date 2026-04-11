//! DAW モードの描画

mod grid;
mod help;
mod history;
mod logs;
mod mixer;
mod patch_select;
mod status;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
    Frame,
};
use status::daw_mode_title;

use super::{AbRepeatState, CacheState, DawApp, DawMode};

/// Pending インジケータのアニメーション 1 フレームの長さ（ミリ秒）
const ANIM_FRAME_MS: u128 = 250;
/// Pending インジケータのアニメーションフレーム数（"." / ".." / "..."）
const ANIM_FRAME_COUNT: u128 = 3;
pub(super) use crate::ui_theme::{
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK,
    MONOKAI_PURPLE, MONOKAI_YELLOW,
};

pub(super) struct CacheRenderSnapshot {
    pub(super) states: Vec<Vec<CacheState>>,
    pub(super) active_render_count: usize,
}

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

fn loop_status_label(mmls: &[String], ab_repeat_state: AbRepeatState) -> Option<String> {
    super::playback::effective_measure_count(mmls).map(|count| {
        let (loop_start_measure_index, loop_end_measure_index) = ab_repeat_state
            .normalized_range(count)
            .unwrap_or((0, count - 1));
        let loop_count = loop_end_measure_index - loop_start_measure_index + 1;
        let label_prefix = if ab_repeat_state == AbRepeatState::Off {
            "loop"
        } else {
            "A-B"
        };
        if loop_count == 1 {
            format!(
                "{label_prefix}: meas{}のみ (1小節)",
                loop_start_measure_index + 1
            )
        } else {
            format!(
                "{label_prefix}: meas{}〜meas{} ({loop_count}小節)",
                loop_start_measure_index + 1,
                loop_end_measure_index + 1
            )
        }
    })
}

fn loop_measure_summary_label(mmls: &[String], ab_repeat_state: AbRepeatState) -> Option<String> {
    super::playback::loop_measure_summary_label(mmls, ab_repeat_state)
}

fn cache_render_snapshot(app: &DawApp) -> CacheRenderSnapshot {
    let cache = app.cache.lock().unwrap();
    let mut active_render_count = 0;
    let states = (0..app.tracks)
        .map(|t| {
            (0..=app.measures)
                .map(|m| {
                    let state = cache[t][m].state.clone();
                    if state == CacheState::Rendering {
                        active_render_count += 1;
                    }
                    state
                })
                .collect()
        })
        .collect();
    CacheRenderSnapshot {
        states,
        active_render_count,
    }
}

pub(super) fn draw(app: &DawApp, f: &mut Frame) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(daw_mode_title(&app.mode))
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
            Constraint::Length(1),
        ])
        .split(inner);

    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let cache_render_snapshot = cache_render_snapshot(app);

    grid::draw_grid(app, f, body_chunks[0], &cache_render_snapshot.states);
    logs::draw_logs(app, f, body_chunks[1]);
    status::draw_status(
        app,
        f,
        chunks[1],
        chunks[2],
        chunks[3],
        chunks[4],
        cache_render_snapshot.active_render_count,
    );

    if app.mode == DawMode::Help {
        match app.help_origin {
            DawMode::Mixer => mixer::draw_mixer(f, app, inner),
            DawMode::History => history::draw_history(f, app, inner),
            DawMode::PatchSelect => patch_select::draw_patch_select(f, app, inner),
            _ => {}
        }
        help::draw_help(f, inner, app.help_origin);
    } else if app.mode == DawMode::Mixer {
        mixer::draw_mixer(f, app, inner);
    } else if app.mode == DawMode::History {
        history::draw_history(f, app, inner);
    } else if app.mode == DawMode::PatchSelect {
        patch_select::draw_patch_select(f, app, inner);
    }
}

#[cfg(test)]
#[path = "../tests/daw/ui.rs"]
mod tests;
