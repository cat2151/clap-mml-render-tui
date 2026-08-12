//! Ctrl+B の BPM 設定 overlay。grid sequencer と loop browser が共有する。
//!
//! 現在の BPM だけ画面ごとに違う（grid はクロックのテンポ、loop は clamp 後の
//! target BPM）ので、そこだけ引数で受け取る。

use ratatui::{
    layout::Alignment,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{format_bpm, BpmInput, BpmMode, BpmRange, MAX_BPM, MIN_BPM};
use crate::{status::base_style, theme::MONOKAI_GREEN, ui::centered_rect_with_size};

const OVERLAY_WIDTH: u16 = 54;

pub fn draw(
    frame: &mut Frame<'_>,
    input: &BpmInput,
    current_bpm: f64,
    mode: BpmMode,
    range: BpmRange,
) {
    let mut lines = vec![
        Line::from(format!(
            "現在: {} {} (自動BPM範囲 {})",
            format_bpm(current_bpm),
            mode.label(),
            range.label(),
        )),
        Line::from(format!(
            "BPM ({MIN_BPM:.0}〜{MAX_BPM:.0}): {}_",
            input.buffer()
        )),
        Line::from("Enter:確定  A:自動BPM  Esc:cancel"),
        Line::from("「80-160」なら1周ごとにその範囲でテンポが変わる"),
    ];
    if let Some(error) = input.error() {
        lines.push(Line::from(error.to_string()));
    }
    let height = (lines.len() as u16).saturating_add(2);
    let area = centered_rect_with_size(OVERLAY_WIDTH.min(frame.area().width), height, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" BPM設定 ")
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_GREEN)),
            ),
        area,
    );
}
