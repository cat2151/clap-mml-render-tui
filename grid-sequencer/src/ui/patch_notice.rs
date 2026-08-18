//! PATCH 欄を押しても selector が開けなかった理由の overlay。
//!
//! 出す理由と消し方は [`crate::patch_notice`] を見ること。ここは描画だけ。

use ratatui::{
    layout::Alignment,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_PINK, MONOKAI_YELLOW},
    ui::centered_text_block_rect,
};

use crate::{patch_notice::PatchUnavailable, GridSequencerScreen};

const TITLE: &str = " 音色選択 ";

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let Some(notice) = screen.patch_notice.as_ref() else {
        return;
    };
    let lines = notice_lines(&notice.reason);
    let area = centered_text_block_rect(f.area(), TITLE, &lines);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(TITLE)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_PINK)),
            ),
        area,
    );
}

/// 1 行目だけ強調する。狭い端末で下が切れても、何が起きたかは残る。
pub(crate) fn notice_lines(reason: &PatchUnavailable) -> Vec<Line<'static>> {
    reason
        .lines()
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let style = if index == 0 {
                base_style().fg(MONOKAI_YELLOW)
            } else {
                base_style()
            };
            Line::from(Span::styled(text, style))
        })
        .collect()
}
