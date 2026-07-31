use ratatui::{
    layout::Alignment,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_YELLOW},
    ui::centered_text_block_rect,
};

const TITLE: &str = " お知らせ ";

/// コード進行データの更新を知らせ、これからアプリを再起動することを伝える。
pub(super) fn draw_overlay(f: &mut Frame<'_>) {
    let lines = notice_lines();
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
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}

fn notice_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "コード進行データが更新されました",
            base_style().fg(MONOKAI_YELLOW),
        )),
        Line::from("反映のためアプリを再起動します…"),
    ]
}
