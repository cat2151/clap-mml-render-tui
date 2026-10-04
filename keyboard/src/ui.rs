use std::time::Instant;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{
    KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardScreen, KeyboardState,
    KeyboardVoicingStatus, NumericInput, NumericInputTarget, PatchPaneFocus,
};
use cmrt_patch_select::ui::draw_plugin_menu;
use cmrt_tui_core::status::base_style;
use cmrt_tui_core::theme::{
    MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK, MONOKAI_PURPLE,
};

mod connection_overlay;
mod controller;
mod effect;
mod guide;
mod help;
mod mml_overlay;
mod note;
mod note_columns;
mod patch_panes;
mod share_notice;

use connection_overlay::draw_connection_overlay;
use controller::{controller_status_lines, controller_status_width};
use effect::{draw_effect_add_overlay, draw_effect_pane};
use guide::{draw_note_guide_overlay, keyboard_help_lines};
use mml_overlay::draw_mml_input_overlay;
use note::{note_playback_mode_line, note_playback_status_line};
use note_columns::{note_column_lines, note_columns_width};
use patch_panes::{draw_patch_selector, PatchSelectorWidths};
use share_notice::draw_share_notice_overlay;

/// keyboard pane の幅の上限。上へ重ねる overlay の上限幅（72 + 枠 2）に合わせる。
const KEYBOARD_PANE_MAX_WIDTH: u16 = 74;
/// keyboard pane の中身の右に空ける桁数。
const KEYBOARD_PANE_MARGIN: usize = 2;
const KEYBOARD_PANE_TITLE: &str = " [KEYBOARD] ";

/// keyboard 画面を描画する。
///
/// patch catalog / voicing 判定の同期は共有ランタイム（`TuiApp`）側の責務なので、
/// 呼び出し前に済ませておくこと。`now` は発音中の音の色付けを実音の時刻に合わせるのに使う。
pub fn draw(
    screen: &mut KeyboardScreen<'_>,
    connection: &KeyboardConnectionStatus,
    now: Instant,
    f: &mut Frame,
) {
    // 「一覧に出ていない音色がある」ことの案内。help 行の上へ、行数ぶんだけ場所を取る。
    // 案内が無いときは 1 行も増えないので、ふだんの見え方は変わらない。
    let catalog_notes = screen.state.patch_catalog.catalog_notes().to_vec();
    let help_height = 3 + u16::try_from(catalog_notes.len()).unwrap_or(0);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(help_height),
        ])
        .split(f.area());
    let keyboard_w = keyboard_pane_width(&screen.state, now);
    let selector_widths = PatchSelectorWidths::new(
        &screen.state.patch_catalog,
        chunks[0].width.saturating_sub(keyboard_w),
    );
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(keyboard_w),
            Constraint::Length(selector_widths.total()),
            Constraint::Fill(1),
        ])
        .split(chunks[0]);

    let keyboard_area = panes[0];
    draw_keyboard(&screen.state, now, f, keyboard_area);
    draw_patch_selector(
        &mut screen.state.patch_catalog,
        &screen.patch_filter,
        &selector_widths,
        f,
        panes[1],
    );
    draw_effect_pane(screen, f, panes[2]);

    let (state, color) = match &connection.phase {
        KeyboardConnectionPhase::Idle => ("server: idle".to_string(), MONOKAI_CYAN),
        KeyboardConnectionPhase::Connecting => ("server: connecting".to_string(), MONOKAI_PURPLE),
        KeyboardConnectionPhase::PatchSetting => {
            ("server: patch setting".to_string(), MONOKAI_PURPLE)
        }
        KeyboardConnectionPhase::Ready => ("server: ready".to_string(), MONOKAI_GREEN),
        KeyboardConnectionPhase::Error(error) => (format!("server error: {error}"), Color::Red),
    };
    let last_send = connection
        .last_send
        .map(format_send_duration)
        .unwrap_or_else(|| "-".to_string());
    let status = format!(
        "transport: SHM | instance: 0 | buffer: x{} | {state} | last send: {last_send} | {}",
        connection.buffer_multiplier,
        voicing_status_text(&connection.voicing)
    );
    f.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[1],
    );
    let mut help_lines: Vec<Line<'_>> = catalog_notes
        .iter()
        .map(|note| Line::from(Span::styled(note.clone(), base_style().fg(MONOKAI_PINK))))
        .collect();
    help_lines.extend(keyboard_help_lines(
        screen.note_guide.presentation(),
        screen.state.navigation_count.value(),
        screen.state.patch_catalog.focus() == PatchPaneFocus::Effect,
    ));
    f.render_widget(Paragraph::new(help_lines).style(base_style()), chunks[2]);
    draw_connection_overlay(connection, f, keyboard_area);
    draw_numeric_input_overlay(
        screen.state.numeric_input(),
        screen.state.cc_number(),
        f,
        keyboard_area,
    );
    draw_mml_input_overlay(&screen.mml_input, f, keyboard_area);
    draw_share_notice_overlay(screen.share_notice(), f, keyboard_area);
    if let Some(menu) = screen.plugin_menu() {
        draw_plugin_menu(menu, screen.state.patch_catalog.filter(), f, chunks[0]);
    }
    draw_effect_add_overlay(screen, f, chunks[0]);
    draw_note_guide_overlay(screen.note_guide.presentation(), f, f.area());
    if screen.help_open() {
        help::draw_overlay(f);
    }
}

/// 周期で変わる値は最大の桁で測るので、和音や操作のモードを変えない限り演奏中も同じ幅になる。
fn keyboard_pane_width(state: &KeyboardState, now: Instant) -> u16 {
    let content = [
        note_playback_mode_line(state).width(),
        note_columns_width(state),
        controller_status_width(state),
        note_playback_status_line(state, now).width(),
        KEYBOARD_PANE_TITLE.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    u16::try_from(content + KEYBOARD_PANE_MARGIN + 2)
        .unwrap_or(u16::MAX)
        .min(KEYBOARD_PANE_MAX_WIDTH)
}

fn draw_keyboard(state: &KeyboardState, now: Instant, f: &mut Frame<'_>, area: Rect) {
    let [pc_key_line, note_line] = note_column_lines(state, now);
    let mut lines = vec![
        note_playback_mode_line(state),
        Line::from(""),
        pc_key_line,
        note_line,
        Line::from(""),
    ];
    lines.extend(controller_status_lines(state));
    lines.push(note_playback_status_line(state, now));
    // 枠の内側に全行が入らないときは、区切りの空行から削る。
    if usize::from(area.height.saturating_sub(2)) < lines.len() {
        lines.remove(4);
        lines.remove(1);
    }

    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(KEYBOARD_PANE_TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_FG)),
        ),
        area,
    );
}

fn draw_numeric_input_overlay(
    input: Option<&NumericInput>,
    cc_number: u8,
    f: &mut Frame<'_>,
    keyboard_area: Rect,
) {
    let Some(input) = input else {
        return;
    };
    let (title, label) = match input.target() {
        NumericInputTarget::CcNumber => (" CC number ", "CC番号を入力".to_string()),
        NumericInputTarget::CcValue => {
            (" CC value ", format!("CC値を入力 (CC#{cc_number} へ送信)"))
        }
    };
    let lines = vec![
        Line::from(format!("{label}: {}_", input.buffer())),
        Line::from("Enter:確定  Esc:cancel"),
    ];
    let width = keyboard_area.width.saturating_sub(2).min(48);
    let height = 4.min(keyboard_area.height);
    let area = cmrt_tui_core::ui::centered_rect_with_size(width, height, keyboard_area);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_GREEN)),
            ),
        area,
    );
}

fn format_send_duration(duration: std::time::Duration) -> String {
    let micros = duration.as_secs_f64() * 1_000_000.0;
    if micros < 1_000.0 {
        format!("{micros:.0} us")
    } else {
        format!("{:.1} ms", micros / 1_000.0)
    }
}

/// ショートカットキーの文字の色。発音中のハイライト（緑）と区別する。
fn shortcut_style() -> Style {
    base_style().fg(MONOKAI_CYAN)
}

/// 巡回する選択肢の色。今の選択肢だけを目立たせる。
fn choice_style(selected: bool) -> Style {
    if selected {
        base_style().fg(MONOKAI_GREEN).add_modifier(Modifier::BOLD)
    } else {
        base_style().fg(MONOKAI_GRAY)
    }
}

/// `label` の中の `key` の文字だけをショートカット色にした span 列。
fn shortcut_label(label: &'static str, key: char) -> Vec<Span<'static>> {
    let Some(at) = label.find(key) else {
        return vec![Span::styled(label, base_style())];
    };
    let end = at + key.len_utf8();
    [
        Span::styled(&label[..at], base_style()),
        Span::styled(&label[at..end], shortcut_style()),
        Span::styled(&label[end..], base_style()),
    ]
    .into_iter()
    .filter(|span| !span.content.is_empty())
    .collect()
}

fn voicing_status_text(status: &KeyboardVoicingStatus) -> String {
    match status {
        KeyboardVoicingStatus::Unavailable => "detect: unavailable".to_string(),
        KeyboardVoicingStatus::Detecting { previous: None } => "detect: probing".to_string(),
        KeyboardVoicingStatus::Detecting {
            previous: Some(report),
        } => format!("{} (probing new patch)", voicing_report_text(report)),
        KeyboardVoicingStatus::Detected(report) => voicing_report_text(report),
        KeyboardVoicingStatus::Cached(voicing) => {
            format!("detect: {} (cached)", voicing_label(*voicing))
        }
    }
}

fn voicing_report_text(report: &cmrt_realtime_play::VoicingReport) -> String {
    let decision = voicing_label(report.decision);
    let probe = voicing_label(report.probe.result);
    let surge = report
        .surge
        .as_ref()
        .map(|surge| format!(" Surge:{}", surge.result))
        .unwrap_or_default();
    let conflict = if report.disagreement { " !" } else { "" };
    format!("detect: {decision} [probe:{probe}{surge}{conflict}]")
}

fn voicing_label(voicing: cmrt_realtime_play::PatchVoicing) -> &'static str {
    match voicing {
        cmrt_realtime_play::PatchVoicing::Mono => "mono",
        cmrt_realtime_play::PatchVoicing::Poly => "poly",
        cmrt_realtime_play::PatchVoicing::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests;
