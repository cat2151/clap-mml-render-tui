use ratatui::{
    layout::Alignment,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui_theme::MONOKAI_YELLOW;

use super::super::status::base_style;

const GUIDE_OVERLAY_WIDTH_PERCENT: u16 = 56;
const GUIDE_OVERLAY_HEIGHT_PERCENT: u16 = 36;

pub(in crate::tui::ui) fn draw_notepad_history_guide(f: &mut Frame) {
    let area = crate::ui_utils::centered_rect(
        GUIDE_OVERLAY_WIDTH_PERCENT,
        GUIDE_OVERLAY_HEIGHT_PERCENT,
        f.area(),
    );
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(vec![
            Line::from("現在の行にはpatch nameがありません。"),
            Line::from("notepad history overlayを開きます。"),
            Line::from("ENTERを押してください"),
        ])
        .style(base_style())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" お知らせ ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_YELLOW)),
        ),
        area,
    );
}
