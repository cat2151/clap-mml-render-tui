//! ビブラート設定 overlay の4項目・単位・現在値と操作を表示する。

use ratatui::{
    layout::{Constraint, Layout},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListState, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG},
    ui::centered_rect_with_size,
};

use crate::GuitarArticulationScreen;

const TITLE: &str = " ビブラート設定 (Shift+V) / フレーズ共通 ";
const STAGES: &str = " 待機 → 直線で立ち上がり → 最終深さを維持 ";
const KEYS: &str = " j/k ↑↓:選択  h/l ←→:変更  Esc:保持して閉じる ";

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    let Some(selected) = screen.vibrato_selected() else {
        return;
    };
    let settings = screen.rules().vibrato_settings();
    let rows = [
        format!(
            " 待機       {:>4} ms  (0〜5000 ms / ±50 ms)",
            settings.delay_ms
        ),
        format!(
            " 立ち上がり {:>4} ms  (0〜5000 ms / ±50 ms)",
            settings.rise_ms
        ),
        format!(" 最終深さ   {:>4}     (0〜127 / ±8)", settings.depth),
        format!(
            " 速度       {:>4}     (0〜127 / ±8 / CC21・uと共通)",
            screen.rules().param(21)
        ),
    ];
    let width = rows
        .iter()
        .map(|row| row.as_str())
        .chain([TITLE, STAGES, KEYS])
        .map(|text| Line::from(text).width() as u16 + 2)
        .max()
        .unwrap_or(0)
        .min(f.area().width);
    let area = centered_rect_with_size(width, 8.min(f.area().height), f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let panes = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(block.inner(area));
    let list = List::new(rows)
        .style(base_style().fg(MONOKAI_FG))
        .highlight_style(cursor_highlight_style(base_style().fg(MONOKAI_FG)));
    let mut state = ListState::default().with_selected(Some(selected));
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    f.render_stateful_widget(list, panes[0], &mut state);
    f.render_widget(Paragraph::new(STAGES).style(base_style()), panes[1]);
    f.render_widget(Paragraph::new(KEYS).style(base_style()), panes[2]);
}

#[cfg(test)]
mod tests;
