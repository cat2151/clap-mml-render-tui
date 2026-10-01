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

use crate::patch_select::PatchSelect;
use crate::plugin_menu::{plugin_mode, PluginMenu, PluginMode};

const TITLE: &str = " plugin solo/mute  a-z:solo  A-Z:mute  同じキー:解除  Esc:閉じる ";

/// selector の menu が開いていれば `area` の中央へ描く。
pub(super) fn draw(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    if let Some(menu) = select.plugin_menu() {
        draw_plugin_menu(menu, select.committed_query(), frame, area);
    }
}

/// `menu` を `area` の中央へ描く。各 plugin の solo / mute の印は `condition` から読む。
pub fn draw_plugin_menu(menu: &PluginMenu, condition: &str, frame: &mut Frame<'_>, area: Rect) {
    let lines: Vec<Line<'_>> = menu
        .items()
        .iter()
        .map(|item| {
            let (mark, color) = match plugin_mode(condition, &item.slug) {
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
