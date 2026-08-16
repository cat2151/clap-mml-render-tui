//! MML オーバーレイから開く音色選択の描画。

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use cmrt_tui_core::{
    status::{base_style, LIST_HIGHLIGHT_SYMBOL},
    text_input::{
        build_query_textarea_widget, single_line_textarea_cursor_position, textarea_value,
    },
    theme::{cursor_highlight_style, MONOKAI_FG, MONOKAI_YELLOW},
    ui::centered_rect,
};

use crate::patch_select::PatchSelect;

const QUERY_TITLE: &str = " 絞り込み  Enter:決定  Esc:取消 ";
const QUERY_PLACEHOLDER: &str = "lead saw";
/// 絞り込み欄の高さ（枠2行 + 入力1行）。
const QUERY_HEIGHT: u16 = 3;

pub(super) fn draw(select: &PatchSelect<'_>, frame: &mut Frame<'_>) {
    let area = centered_rect(80, 70, frame.area());
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(QUERY_HEIGHT), Constraint::Min(1)])
        .split(area);
    draw_query(select, frame, chunks[0]);
    draw_list(select, frame, chunks[1]);
}

fn draw_query(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let textarea = select.query_textarea();
    let value = textarea_value(textarea);
    frame.render_widget(
        &build_query_textarea_widget(
            textarea,
            &value,
            QUERY_TITLE,
            QUERY_PLACEHOLDER,
            MONOKAI_YELLOW,
        ),
        area,
    );
    // 音色選択が開いている間、端末のカーソルは絞り込み欄にある。
    // MML 入力欄より後に描くので、こちらの指定が残る。
    frame.set_cursor_position(single_line_textarea_cursor_position(area, textarea));
}

fn draw_list(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(list_title(select))
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_YELLOW));
    let page_size = usize::from(block.inner(area).height);
    select.set_page_size(page_size);

    let items = select
        .filtered()
        .iter()
        .map(|patch| ListItem::new(patch.as_str()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select((!items.is_empty()).then_some(select.cursor()));
    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        area,
        &mut state,
    );
}

fn list_title(select: &PatchSelect<'_>) -> String {
    format!(" 音色 ({}/{}) ", select.filtered().len(), select.total())
}
