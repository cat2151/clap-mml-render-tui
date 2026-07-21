use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{status::base_style, status::visible_list_page_size, LIST_HIGHLIGHT_SYMBOL};
use crate::tui::keyboard::{
    KeyboardConnectionPhase, KeyboardPatchCatalogStatus, KeyboardState, KeyboardVoicingStatus,
    ModulationMode, NumericInput, NumericInputTarget, PitchBendMode, VelocityMode, KEYBOARD_NOTES,
};
use crate::tui::TuiApp;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_PURPLE};

mod guide;
mod mml_overlay;
mod note;

use guide::{draw_note_guide_overlay, keyboard_help_lines};
use mml_overlay::draw_mml_input_overlay;
use note::note_playback_status_text;

pub(super) fn draw(app: &mut TuiApp<'_>, f: &mut Frame) {
    app.sync_keyboard_patch_catalog();
    app.sync_keyboard_voicing_detection();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(f.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
        ])
        .split(chunks[0]);

    draw_keyboard(app, f, panes[0]);
    draw_patch_catalog(app, f, panes[1], panes[2]);

    let connection = app.keyboard_connection_status();
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
        "transport: {} | buffer: x{} | {state} | {} | last send: {last_send}",
        connection.transport.label(),
        connection.buffer_multiplier,
        voicing_status_text(&connection.voicing)
    );
    f.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[1],
    );
    let keyboard_help = keyboard_help_lines(
        app.keyboard.note_guide.presentation(),
        app.keyboard.state.navigation_count.value(),
    );
    f.render_widget(Paragraph::new(keyboard_help).style(base_style()), chunks[2]);
    draw_connection_overlay(&connection.phase, f, panes[0]);
    draw_numeric_input_overlay(
        app.keyboard.state.numeric_input(),
        app.keyboard.state.cc_number(),
        f,
        panes[0],
    );
    draw_mml_input_overlay(&app.keyboard.mml_input, f, panes[0]);
    draw_note_guide_overlay(app.keyboard.note_guide.presentation(), f, f.area());
}

fn draw_keyboard(app: &TuiApp<'_>, f: &mut Frame<'_>, area: Rect) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "PC key:  c   d   e   f   g   a   b",
            base_style(),
        )),
        Line::from(Span::styled(
            "Note:    C4  D4  E4  F4  G4  A4  B4",
            base_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            active_notes_text(&app.keyboard.state),
            base_style(),
        )),
        Line::from(Span::styled(
            format!(
                "Patch: {}",
                app.keyboard.state.patch().unwrap_or("init saw")
            ),
            base_style(),
        )),
        Line::from(Span::styled(
            controller_status_text(&app.keyboard.state),
            base_style(),
        )),
        Line::from(Span::styled(
            note_playback_status_text(&app.keyboard.state),
            base_style(),
        )),
    ];
    if area.height < 8 {
        lines.remove(0);
        lines.remove(2);
    }
    debug_assert_eq!(KEYBOARD_NOTES.len(), 7);

    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" [KEYBOARD] keyboard mode  1-9:count ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn draw_patch_catalog(
    app: &mut TuiApp<'_>,
    f: &mut Frame<'_>,
    category_area: Rect,
    patch_area: Rect,
) {
    let catalog = &mut app.keyboard.state.patch_catalog;
    let status = catalog.status().clone();
    let categories = catalog.categories().to_vec();
    let selected_category_index = catalog.selected_category_index();
    let selected_patch_index = catalog.selected_patch_index();
    let patches = catalog
        .selected_category()
        .map(|category| category.patches.clone())
        .unwrap_or_default();

    let category_items = if categories.is_empty() {
        vec![ListItem::new(catalog_message(&status))]
    } else {
        categories
            .iter()
            .map(|category| {
                ListItem::new(format!("{} ({})", category.name, category.patches.len()))
            })
            .collect()
    };
    let patch_items = if patches.is_empty() {
        vec![ListItem::new(if categories.is_empty() {
            catalog_message(&status)
        } else {
            "カテゴリー未選択".to_string()
        })]
    } else {
        patches.iter().cloned().map(ListItem::new).collect()
    };

    catalog.sync_list_states(
        visible_list_page_size(category_area),
        visible_list_page_size(patch_area),
    );
    let category_title = format!(
        " Categories ({}/{}) ",
        selected_category_index.map(|index| index + 1).unwrap_or(0),
        categories.len()
    );
    let patch_title = format!(
        " Patches ({}/{}) ",
        selected_patch_index.map(|index| index + 1).unwrap_or(0),
        patches.len()
    );
    let highlight = cursor_highlight_style(base_style());

    f.render_stateful_widget(
        List::new(category_items)
            .style(base_style())
            .highlight_style(highlight)
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(category_title)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        category_area,
        catalog.category_list_state_mut(),
    );
    f.render_stateful_widget(
        List::new(patch_items)
            .style(base_style())
            .highlight_style(highlight)
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(patch_title)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        patch_area,
        catalog.patch_list_state_mut(),
    );
}

fn catalog_message(status: &KeyboardPatchCatalogStatus) -> String {
    match status {
        KeyboardPatchCatalogStatus::Loading => "パッチを読み込み中...".to_string(),
        KeyboardPatchCatalogStatus::NotConfigured => {
            "patches_dirs が設定されていません".to_string()
        }
        KeyboardPatchCatalogStatus::Ready => "パッチが見つかりません".to_string(),
        KeyboardPatchCatalogStatus::Error(error) => format!("読み込み失敗: {error}"),
    }
}

fn draw_connection_overlay(
    phase: &KeyboardConnectionPhase,
    f: &mut Frame<'_>,
    keyboard_area: Rect,
) {
    let (title, lines, border_color, height) = match phase {
        KeyboardConnectionPhase::Ready => return,
        KeyboardConnectionPhase::Idle | KeyboardConnectionPhase::Connecting => (
            " server connection ",
            vec![
                Line::from("connecting..."),
                Line::from("notes unavailable until ready"),
            ],
            MONOKAI_PURPLE,
            5,
        ),
        KeyboardConnectionPhase::PatchSetting => (
            " patch setting ",
            vec![
                Line::from("patch setting..."),
                Line::from("notes unavailable until ready"),
            ],
            MONOKAI_PURPLE,
            5,
        ),
        KeyboardConnectionPhase::Error(error) => (
            " server connection error ",
            vec![
                Line::from(format!("server error: {error}")),
                Line::from("r:retry  n:notepad  w:DAW  q:quit"),
            ],
            Color::Red,
            7,
        ),
    };
    let width = keyboard_area.width.saturating_sub(2).min(72);
    let height = height.min(keyboard_area.height);
    let area = crate::ui_utils::centered_rect_with_size(width, height, keyboard_area);
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
                    .border_style(base_style().fg(border_color)),
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
    let area = crate::ui_utils::centered_rect_with_size(width, height, keyboard_area);
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

fn controller_status_text(state: &KeyboardState) -> String {
    let velocity = match state.velocity_mode() {
        VelocityMode::Periodic => format!("cyc({})", state.velocity()),
        VelocityMode::Normal | VelocityMode::Accent => state.velocity().to_string(),
    };
    let modulation = match state.modulation_mode() {
        ModulationMode::Off => "OFF",
        ModulationMode::On => "ON",
        ModulationMode::Periodic => "CYC",
    };
    let pitch_bend = match state.pitch_bend_mode() {
        PitchBendMode::Idle => "-",
        PitchBendMode::Max => "+8191",
        PitchBendMode::Min => "-8192",
        PitchBendMode::CenterAfterMax
        | PitchBendMode::CenterAfterMin
        | PitchBendMode::CenterAfterCycle => "0",
        PitchBendMode::Periodic => "CYC",
    };
    let cc = if state.cc_periodic_on() {
        format!("{} cyc", state.cc_number())
    } else {
        state.cc_number().to_string()
    };
    let mut text = format!("Vel: {velocity}  Mod: {modulation}  PB: {pitch_bend}  CC#: {cc}");
    if let Some((drawn, total)) = state.combo_progress() {
        text.push_str(&format!("  Combo: {drawn}/{total}"));
    }
    text
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

fn voicing_report_text(report: &crate::realtime_play::VoicingReport) -> String {
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

fn voicing_label(voicing: crate::realtime_play::PatchVoicing) -> &'static str {
    match voicing {
        crate::realtime_play::PatchVoicing::Mono => "mono",
        crate::realtime_play::PatchVoicing::Poly => "poly",
        crate::realtime_play::PatchVoicing::Unknown => "unknown",
    }
}

fn active_notes_text(state: &crate::tui::keyboard::KeyboardState) -> String {
    if state.held().is_empty() {
        return "Active: -".to_string();
    }
    let names = state
        .held()
        .iter()
        .map(|note| note.name)
        .collect::<Vec<_>>()
        .join(" ");
    format!("Active: {names}")
}

#[cfg(test)]
#[path = "keyboard_tests.rs"]
mod tests;
