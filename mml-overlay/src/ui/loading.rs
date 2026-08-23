//! patch load が長引くときの中央表示。

use ratatui::{
    layout::Alignment,
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    theme::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG},
    ui::centered_rect_with_size,
};

use crate::MmlOverlaySenderStatus;

const WIDTH: u16 = 32;
const HEIGHT: u16 = 3;

pub(super) fn draw(status: &MmlOverlaySenderStatus, frame: &mut Frame<'_>) {
    if !status.is_loading() {
        return;
    }
    let screen = frame.area();
    let area = centered_rect_with_size(WIDTH.min(screen.width), HEIGHT.min(screen.height), screen);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("Now loading...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}
