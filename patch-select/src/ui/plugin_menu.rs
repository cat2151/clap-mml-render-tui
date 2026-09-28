//! plugin solo/mute overlay の描画。

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_FG, MONOKAI_YELLOW},
    ui::centered_text_block_rect,
};

use crate::patch_select::{PatchSelect, PluginMode};

const TITLE: &str = " plugin solo/mute  a-z:solo  A-Z:mute  同じキー:解除  Esc:閉じる ";

/// menu が開いていれば `area` の中央へ描く。
pub(super) fn draw(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let Some(menu) = select.plugin_menu() else {
        return;
    };
    let lines: Vec<Line<'_>> = menu
        .items()
        .iter()
        .map(|item| {
            let (mark, color) = match select.plugin_mode(&item.slug) {
                Some(PluginMode::Solo) => ("solo", MONOKAI_YELLOW),
                Some(PluginMode::Mute) => ("mute", MONOKAI_YELLOW),
                None => ("", MONOKAI_FG),
            };
            Line::from(format!(" {}  {:<12}{mark:<4} ", item.key, item.slug))
                .style(base_style().fg(color))
        })
        .collect();
    let menu_area = centered_text_block_rect(area, TITLE, &lines);
    frame.render_widget(Clear, menu_area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_YELLOW)),
        ),
        menu_area,
    );
}
