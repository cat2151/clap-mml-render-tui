//! grid sequencer 画面の描画。
//!
//! 上から「コード進行1行（chord mode 中だけ）/ NOTE grid / CC1 grid /
//! VELOCITY grid / ステータス1行 / キーバインド1行」の縦分割で、help は最後に
//! overlay として重ねる。

use ratatui::{layout::Rect, widgets::Paragraph, Frame};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_YELLOW},
};

use crate::{
    GridConnectionPhase, GridConnectionStatus, GridSequencerScreen, BPM, GRID_STEPS, STEP_INTERVAL,
};

mod chord_line;
mod grid;
mod help;
pub mod layout;
mod patch_selector;
mod pattern_list;
mod progress;
mod restart_notice;
mod value_grid;

/// 描画と mouse の当たり判定が同じ矩形を見るための、唯一の layout の作り方。
///
/// grid は中央寄せなので左端が端末幅で動く。片方だけ別の式で組むと、見えている
/// セルと当たるセルがずれる。
pub(crate) fn layout_for(screen: &GridSequencerScreen, area: Rect) -> layout::GridSequencerLayout {
    let visible_rows = screen.state.visible_note_rows();
    layout::GridSequencerLayout::new(
        area,
        visible_rows.len(),
        screen.state.instance_count(),
        visible_rows.len(),
        screen.chord_line_visible(),
        screen.state.chord().is_some(),
    )
}

pub fn draw(screen: &GridSequencerScreen, connection: &GridConnectionStatus, f: &mut Frame<'_>) {
    let chord_line = chord_line::line(screen);
    let layout = layout_for(screen, f.area());
    if let (Some(line), Some(area)) = (chord_line, layout.chord_line) {
        f.render_widget(Paragraph::new(line).style(base_style()), area);
    }
    draw_grids(screen, connection, f, &layout);
    if let Some(area) = layout.pattern_list {
        pattern_list::draw(screen, f, area);
    }
    f.render_widget(
        status_line(screen, connection, layout.status.width),
        layout.status,
    );
    f.render_widget(
        Paragraph::new(help::KEYBIND_TEXT).style(base_style().fg(MONOKAI_GRAY)),
        layout.keybind,
    );
    if screen.patch_selector.is_some() {
        patch_selector::draw(f, screen);
    }
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

/// NOTE grid を必要行数ぶん先に確保し、残りを CC1 と VELOCITY で2等分する。
///
/// 低い端末でも両方のタイトルが必ず出るよう、片方に必要行数を全部渡すことはしない。
fn draw_grids(
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
    f: &mut Frame<'_>,
    layout: &layout::GridSequencerLayout,
) {
    grid::draw(screen, connection, f, layout.note);
    draw_value_grid(
        f,
        layout.cc1,
        value_grid_title("CC1 Modulation", screen.state.cc1_pattern_label()),
        screen.state.cc1_display(),
        &(0..screen.state.instance_count()).collect::<Vec<_>>(),
        screen,
        connection,
    );
    let velocity_display = screen.state.visible_velocity_display();
    let velocity_instances = screen
        .state
        .visible_note_rows()
        .into_iter()
        .map(|row| row.address.instance)
        .collect::<Vec<_>>();
    draw_value_grid(
        f,
        layout.velocity,
        value_grid_title("Velocity", screen.state.velocity_pattern_label()),
        &velocity_display,
        &velocity_instances,
        screen,
        connection,
    );
}

/// 小節ごとの抽選パターンはタイトルへ添える。実発音中の小節のものなので、
/// grid の値と同じタイミングで切り替わる。再生前は添えない。
fn value_grid_title(name: &str, pattern: Option<&str>) -> String {
    match pattern {
        Some(pattern) => format!(" {name} [{pattern}] "),
        None => format!(" {name} "),
    }
}

/// 抽選値の grid。強調するのは最大値（CC1 も velocity も 127）のセル。
fn draw_value_grid(
    f: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    title: String,
    display: &[[Option<u8>; GRID_STEPS]],
    row_instances: &[usize],
    screen: &GridSequencerScreen,
    connection: &GridConnectionStatus,
) {
    if area.height == 0 {
        return;
    }
    value_grid::draw(
        f,
        area,
        title,
        display,
        screen.state.step_index(),
        connection,
        row_instances,
    );
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
            " SHM {} | buffer x{multiplier} ({latency_ms:.0}ms) {mode} | underrun {} frames | {} instances / {} note lanes | BPM {} 1/16={:.1}ms | step {:>2}/{} | GR {:.1} dB | {} ",
            connection.label(),
            connection.underrun_frames,
            screen.track_count(),
            screen.state.visible_lane_count(),
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
            " SHM {} | buf x{multiplier} {latency_ms:.0}ms{compact_mode} | underrun {}f | {}i/{}l | {}bpm | step {}/{} | GR{:.1} | {} ",
            connection.label(),
            connection.underrun_frames,
            screen.track_count(),
            screen.state.visible_lane_count(),
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
