//! レイアウト系 UI ユーティリティ（画面横断で共有）。

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
};

const BLOCK_BORDER_SIZE: usize = 2;

/// 指定した割合で中央に配置した矩形を返す。ポップアップ表示に利用する。
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
pub fn centered_rect_with_size(width: u16, height: u16, area: Rect) -> Rect {
    if area.width == 0 || area.height == 0 {
        return area;
    }

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
pub fn centered_text_block_rect(area: Rect, title: &str, lines: &[Line<'_>]) -> Rect {
    let content_width = lines.iter().map(Line::width).max().unwrap_or(0);
    let title_width = Line::from(title).width();
    let raw_width = content_width
        .max(title_width)
        .saturating_add(BLOCK_BORDER_SIZE);
    let raw_height = lines.len().saturating_add(BLOCK_BORDER_SIZE);
    let clamped_width = raw_width.min(area.width as usize);
    let clamped_height = raw_height.min(area.height as usize);

    centered_rect_with_size(clamped_width as u16, clamped_height as u16, area)
}
