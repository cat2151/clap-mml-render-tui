//! grid sequencer 画面の描画。
//!
//! 上から「コード進行1行（chord mode 中だけ）/ NOTE grid / CC1 grid /
//! ステータス1行 / キーバインド1行」の縦分割で、help は最後に overlay として重ねる。

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

mod cc1_grid;
mod chord_line;
mod grid;
mod help;
mod progress;
mod restart_notice;

pub fn draw(screen: &GridSequencerScreen, connection: &GridConnectionStatus, f: &mut Frame<'_>) {
    // コード進行行は出すときだけ確保する。off のときの1行は grid に使う。
    let chord_line = chord_line::line(screen);
    let mut constraints = Vec::with_capacity(4);
    if chord_line.is_some() {
        constraints.push(Constraint::Length(1));
    }
    constraints.extend([
        Constraint::Min(5),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());
    let grid_index = if let Some(chord_line) = chord_line {
        f.render_widget(Paragraph::new(chord_line).style(base_style()), chunks[0]);
        1
    } else {
        0
    };
    let (status_area, keybind_area) = (chunks[grid_index + 1], chunks[grid_index + 2]);
    draw_grids(screen, connection, f, chunks[grid_index]);
    f.render_widget(
        status_line(screen, connection, status_area.width),
        status_area,
    );
    f.render_widget(
        Paragraph::new(help::KEYBIND_TEXT).style(base_style().fg(MONOKAI_GRAY)),
        keybind_area,
    );
    // 準備中とエラーは中央 overlay で知らせる。help は常に最前面。
    if connection.is_preparing() || connection.error_message().is_some() {
        progress::draw_overlay(f, connection, screen.track_count());
    }
    if screen.restart_notice_open() {
        restart_notice::draw_overlay(f);
    }
    if screen.help_open {
        help::draw_overlay(f, screen.track_count());
    }
}

/// NOTE grid を必要行数ぶん先に確保し、残りを CC1 grid に割り当てる。
fn draw_grids(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    f: &mut Frame<'_>,
    area: ratatui::layout::Rect,
) {
    // 上下のborder 2行に、header 1行と全track行が必要。
    let note_height = u16::try_from(screen.track_count() + 3)
        .unwrap_or(u16::MAX)
        .min(area.height);
    let note_area = ratatui::layout::Rect {
        height: note_height,
        ..area
    };
    grid::draw(screen, connection, f, note_area);

    let cc1_height = area.height.saturating_sub(note_height);
    if cc1_height == 0 {
        return;
    }
    let cc1_area = ratatui::layout::Rect {
        y: area.y.saturating_add(note_height),
        height: cc1_height,
        ..area
    };
    cc1_grid::draw(screen, connection, f, cc1_area);
}

fn status_line(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    width: u16,
) -> Paragraph<'static> {
    let color = match &connection.phase {
        GridConnectionPhase::Ready => MONOKAI_GREEN,
        GridConnectionPhase::Error(_) => MONOKAI_PINK,
        GridConnectionPhase::Idle
        | GridConnectionPhase::Connecting
        | GridConnectionPhase::WaitingForPatches
        | GridConnectionPhase::PatchSetting => MONOKAI_YELLOW,
    };
    let multiplier = connection.buffer_multiplier;
    let latency_ms = screen.buffer_latency_ms(multiplier);
    // 裏読みをやめているかどうかは、音の途切れ方の説明そのものなので必ず出す。
    let mode = if screen.single_buffering() {
        "single"
    } else {
        "auto"
    };
    let compact_mode = if screen.single_buffering() { " sb" } else { "" };
    let text = if width >= 160 {
        format!(
            " SHM {} | buffer x{multiplier} ({latency_ms:.0}ms) {mode} | underrun {} frames | {} instances | BPM {} 1/16={:.1}ms | step {:>2}/{} | GR {:.1} dB | {} ",
            connection.label(),
            connection.underrun_frames,
            screen.track_count(),
            BPM,
            STEP_INTERVAL.as_secs_f64() * 1000.0,
            screen.state.step_index() + 1,
            GRID_STEPS,
            connection.limiter_reduction_db,
            screen.patch_status.label(),
        )
    } else {
        // 90桁でも patch 状態まで出し切れるよう、倍率とレイテンシは最短表記にする。
        format!(
            " SHM {} | buf x{multiplier} {latency_ms:.0}ms{compact_mode} | underrun {}f | {}tr | {}bpm | step {}/{} | GR{:.1} | {} ",
            connection.label(),
            connection.underrun_frames,
            screen.track_count(),
            BPM,
            screen.state.step_index() + 1,
            GRID_STEPS,
            connection.limiter_reduction_db,
            compact_patch_status(screen),
        )
    };
    Paragraph::new(text).style(base_style().fg(color))
}

fn compact_patch_status(screen: &GridSequencerScreen) -> String {
    match &screen.patch_status {
        crate::GridPatchStatus::Ready(count) => format!("p:{count}"),
        crate::GridPatchStatus::Loading => "p:load".to_string(),
        crate::GridPatchStatus::NotConfigured => "p:none".to_string(),
        crate::GridPatchStatus::Err(_) => "p:err".to_string(),
    }
}

#[cfg(test)]
mod tests;
