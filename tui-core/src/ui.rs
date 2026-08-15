//! レイアウト系 UI ユーティリティ（画面横断で共有）。

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::theme::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_YELLOW};

#[cfg(test)]
mod tests;

const BLOCK_BORDER_SIZE: usize = 2;

/// 現在frameの全セルを共通背景で所有し、前画面の未描画セルを残さない。
pub fn draw_frame_background(f: &mut Frame<'_>) {
    f.render_widget(
        Block::default().style(crate::status::base_style()),
        f.area(),
    );
}

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

/// 音出し確認ガイドの中央オーバーレイを描画する（notepad / keyboard / DAW 共通）。
pub fn draw_sound_check_guide_overlay(f: &mut Frame<'_>, area: Rect, message: &str) {
    let base_style = Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG);
    let width = area.width.saturating_sub(2).min(72);
    let height = 5.min(area.height);
    let overlay_area = centered_rect_with_size(width, height, area);
    f.render_widget(Clear, overlay_area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            message.to_owned(),
            base_style.fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(base_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 音出し確認 ")
                .style(base_style)
                .border_style(base_style.fg(MONOKAI_CYAN)),
        ),
        overlay_area,
    );
}
