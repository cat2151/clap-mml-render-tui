//! selector を開けていない理由（一覧の Loading 待ち等）の中央表示。入力欄なしで開いたとき、
//! 待っている間に何も描かれないと `t` が効かなかったように見える。

use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    theme::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_PINK, MONOKAI_YELLOW},
    ui::centered_rect_with_size,
};

use crate::PatchCatalogNotice;

const WIDTH: u16 = 60;
const HEIGHT: u16 = 3;

pub(super) fn draw(notice: &PatchCatalogNotice, frame: &mut Frame<'_>) {
    let screen = frame.area();
    let area = centered_rect_with_size(WIDTH.min(screen.width), HEIGHT.min(screen.height), screen);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(notice.message())
            .alignment(Alignment::Center)
            .style(Style::default().fg(color(notice)).bg(MONOKAI_BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 音色 ")
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}

fn color(notice: &PatchCatalogNotice) -> Color {
    match notice {
        PatchCatalogNotice::Loading => MONOKAI_YELLOW,
        PatchCatalogNotice::Empty | PatchCatalogNotice::Error(_) => MONOKAI_PINK,
    }
}
