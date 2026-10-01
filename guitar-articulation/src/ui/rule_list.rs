//! 奏法リスト overlay（`t`）。カーソル列の列ルールを 1 行 1 つ並べ、選んでいる行を反転する。
//! 行頭の記号は matrix のルールの段と同じ（● 効いている / 灰色の - ON だが効かない）。

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG},
    ui::centered_rect_with_size,
};

use super::{matrix::rule_cell, RULE_ROWS};
use crate::GuitarArticulationScreen;

const HINT: &str = "j/k:選択 Enter:ON/OFF Esc:閉じる";

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    let Some(selected) = screen.rule_list_selected() else {
        return;
    };
    let column = screen.cursor();
    let title = format!(" 奏法リスト 列 {}  {HINT} ", column + 1);
    let items: Vec<ListItem> = RULE_ROWS
        .iter()
        .map(|(rule, key, name)| {
            let (mark, style) = rule_cell(screen, column, *rule);
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(mark, style),
                Span::raw(format!(" {key}:{name}")),
            ]))
        })
        .collect();
    let width = (Line::from(title.as_str()).width() as u16 + 2).min(f.area().width);
    let height = (RULE_ROWS.len() as u16 + 2).min(f.area().height);
    let area: Rect = centered_rect_with_size(width, height, f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let list = List::new(items)
        .block(block)
        .style(base_style().fg(MONOKAI_FG))
        .highlight_style(cursor_highlight_style(base_style().fg(MONOKAI_FG)));
    let mut state = ListState::default().with_selected(Some(selected));
    f.render_widget(Clear, area);
    f.render_stateful_widget(list, area, &mut state);
}
