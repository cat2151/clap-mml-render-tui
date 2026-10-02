//! `y` でコピーした共有コマンドを知らせる中央 overlay（keyboard）。

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::base_style;
use cmrt_tui_core::theme::MONOKAI_GREEN;

pub(super) const SHARE_NOTICE_MESSAGE: &str = "クリップボードにコピーしました";

/// コマンド全文を、枠の内側の幅で文字単位に折り返して出す。
pub(super) fn draw_share_notice_overlay(
    command: Option<&str>,
    f: &mut Frame<'_>,
    keyboard_area: Rect,
) {
    let Some(command) = command else {
        return;
    };
    let width = keyboard_area.width.saturating_sub(2).min(72);
    let inner_width = usize::from(width.saturating_sub(2)).max(1);
    let mut lines = vec![Line::from(SHARE_NOTICE_MESSAGE)];
    lines.extend(
        wrap_by_width(command, inner_width)
            .into_iter()
            .map(|row| Line::from(Span::styled(row, base_style().fg(MONOKAI_GREEN)))),
    );
    let height = u16::try_from(lines.len() + 2)
        .unwrap_or(u16::MAX)
        .min(keyboard_area.height);
    let area = cmrt_tui_core::ui::centered_rect_with_size(width, height, keyboard_area);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" share ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_GREEN)),
        ),
        area,
    );
}

/// 表示幅 `width` ごとに区切る。単語の切れ目は見ない（全文がそのまま並ぶように）。
fn wrap_by_width(text: &str, width: usize) -> Vec<String> {
    let mut rows = vec![String::new()];
    let mut row_width = 0;
    for ch in text.chars() {
        let ch_width = Span::raw(ch.to_string()).width();
        if row_width + ch_width > width && row_width > 0 {
            rows.push(String::new());
            row_width = 0;
        }
        rows.last_mut().expect("rows is never empty").push(ch);
        row_width += ch_width;
    }
    rows
}
