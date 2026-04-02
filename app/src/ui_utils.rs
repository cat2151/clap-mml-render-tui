//! UI ユーティリティ（TUI / DAW 共通）

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
};

const BLOCK_BORDER_SIZE: usize = 2;

/// 指定した割合で中央に配置した矩形を返す。ポップアップ表示に利用する。
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let px = percent_x.min(100);
    let py = percent_y.min(100);
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ])
        .split(v[1])[1]
}

/// 指定したサイズで中央に配置した矩形を返す。
pub(crate) fn centered_rect_with_size(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.max(1).min(area.width);
    let height = height.max(1).min(area.height);
    Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    )
}

/// テキスト行数と最大描画幅に合わせた Block 用の中央配置矩形を返す。
pub(crate) fn centered_text_block_rect(area: Rect, title: &str, lines: &[Line<'_>]) -> Rect {
    let content_width = lines.iter().map(Line::width).max().unwrap_or(0);
    let title_width = Line::from(title).width();
    centered_rect_with_size(
        content_width
            .max(title_width)
            .saturating_add(BLOCK_BORDER_SIZE) as u16,
        lines.len().saturating_add(BLOCK_BORDER_SIZE) as u16,
        area,
    )
}
