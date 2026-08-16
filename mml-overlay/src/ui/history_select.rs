//! MML オーバーレイから開くフレーズ履歴の描画。
//!
//! notepad 画面の履歴と同じく、履歴とお気に入りを左右に並べる。絞り込み欄が
//! 左右キーを使うので、ペインの行き来は Tab に割り当てている。

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
    theme::{cursor_highlight_style, MONOKAI_DARK_GRAY, MONOKAI_FG, MONOKAI_YELLOW},
    ui::centered_rect,
};

use crate::history_select::{HistoryPane, HistorySelect};

const QUERY_TITLE: &str = " 絞り込み  Tab:ペイン切替  Enter:決定  Esc:取消 ";
const QUERY_PLACEHOLDER: &str = "cde";
/// 絞り込み欄の高さ（枠2行 + 入力1行）。
const QUERY_HEIGHT: u16 = 3;

pub(super) fn draw(select: &HistorySelect<'_>, frame: &mut Frame<'_>) {
    let area = centered_rect(90, 80, frame.area());
    frame.render_widget(Clear, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(QUERY_HEIGHT), Constraint::Min(1)])
        .split(area);
    draw_query(select, frame, rows[0]);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);
    draw_pane(select, frame, panes[0], HistoryPane::History, "履歴");
    if panes.len() > 1 {
        draw_pane(
            select,
            frame,
            panes[1],
            HistoryPane::Favorites,
            "お気に入り",
        );
    }
}

fn draw_query(select: &HistorySelect<'_>, frame: &mut Frame<'_>, area: Rect) {
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
    // 履歴が開いている間、端末のカーソルは絞り込み欄にある。
    // MML 入力欄より後に描くので、こちらの指定が残る。
    frame.set_cursor_position(single_line_textarea_cursor_position(area, textarea));
}

fn draw_pane(
    select: &HistorySelect<'_>,
    frame: &mut Frame<'_>,
    area: Rect,
    pane: HistoryPane,
    label: &str,
) {
    let focused = select.focus() == pane;
    let border_color = if focused {
        MONOKAI_YELLOW
    } else {
        MONOKAI_DARK_GRAY
    };
    let items = select.items(pane);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " {label} ({}/{}) ",
            items.len(),
            select.total(pane)
        ))
        .style(base_style())
        .border_style(base_style().fg(border_color));
    if focused {
        select.set_page_size(usize::from(block.inner(area).height));
    }

    let list_items = items
        .iter()
        .map(|line| ListItem::new(line.as_str()))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    // カーソルは見ているペインにだけ出す。どちらが効いているかを取り違えないため。
    state.select((focused && !list_items.is_empty()).then_some(select.cursor(pane)));
    frame.render_stateful_widget(
        List::new(list_items)
            .block(block)
            .highlight_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        area,
        &mut state,
    );
}
