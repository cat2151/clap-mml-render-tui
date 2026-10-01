//! パラメータ overlay（`u`）。行全体のパラメータを 1 行 1 つ並べ、選んでいる行を反転する。
//! 既定と違う値の行は頭に ● を付ける。

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GREEN},
    ui::centered_rect_with_size,
};

use crate::{GuitarArticulationScreen, PARAMS, PARAM_STEP};

pub(super) fn draw_overlay(f: &mut Frame<'_>, screen: &GuitarArticulationScreen) {
    let Some(selected) = screen.param_list_selected() else {
        return;
    };
    let title = format!(" パラメータ(行全体)  j/k:選択 h/l:±{PARAM_STEP} Esc:閉じる ");
    let items: Vec<ListItem> = PARAMS
        .iter()
        .map(|param| {
            let value = screen.rules().param(param.cc);
            let mark = if value == param.default { " " } else { "●" };
            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(mark, base_style().fg(MONOKAI_GREEN)),
                Span::raw(format!(
                    " CC{:<3} {:<13} {value:>3}  (既定 {})",
                    param.cc, param.name, param.default
                )),
            ]))
        })
        .collect();
    let width = (Line::from(title.as_str()).width() as u16 + 2).min(f.area().width);
    let height = (PARAMS.len() as u16 + 2).min(f.area().height);
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
