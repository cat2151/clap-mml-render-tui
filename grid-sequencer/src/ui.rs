//! grid sequencer 画面の描画。
//!
//! 上から「grid 本体 / ステータス1行 / キーバインド1行」の縦3分割で、
//! help は最後に overlay として重ねる。

use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::Paragraph,
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW},
};

use crate::{
    GridConnectionPhase, GridConnectionStatus, GridSequencerScreen, BPM, GRID_STEPS, STEP_INTERVAL,
};

mod grid;
mod help;

pub fn draw(screen: &GridSequencerScreen, connection: &GridConnectionStatus, f: &mut Frame<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    grid::draw(screen, f, chunks[0]);
    f.render_widget(status_line(screen, connection), chunks[1]);
    f.render_widget(
        Paragraph::new(help::KEYBIND_TEXT).style(base_style().fg(MONOKAI_GRAY)),
        chunks[2],
    );
    if screen.help_open {
        help::draw_overlay(f);
    }
}

fn status_line(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
) -> Paragraph<'static> {
    let color = match &connection.phase {
        GridConnectionPhase::Ready => MONOKAI_GREEN,
        GridConnectionPhase::Error(_) => MONOKAI_PINK,
        GridConnectionPhase::Idle
        | GridConnectionPhase::Connecting
        | GridConnectionPhase::PatchSetting => MONOKAI_YELLOW,
    };
    let text = format!(
        " {} {} | BPM {} 1/16={:.1}ms | step {:>2}/{} | patch {} | {} ",
        connection.transport.label(),
        connection.phase.label(),
        BPM,
        STEP_INTERVAL.as_secs_f64() * 1000.0,
        screen.state.step_index() + 1,
        GRID_STEPS,
        screen.state.sound_patch().unwrap_or("-"),
        screen.patch_status.label(),
    );
    Paragraph::new(text).style(base_style().fg(color))
}

#[cfg(test)]
mod tests;
