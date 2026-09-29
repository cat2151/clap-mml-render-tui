//! effect chain overlay の描画。chain 一覧か、query 欄 + category / kind / list 3 pane を出す。

use std::cell::Cell;

use cmrt_core::AudioEffectCatalog;
use cmrt_patch_select::ui::scroll_offset;
use cmrt_tui_core::theme::{
    cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{messages as message, stage_label, EffectAddPane, EffectChainEditor};

/// host が決める、overlay の見せ方。
pub struct EffectChainView<'a> {
    /// 追加・差し替えの 3 pane を出すか（`false` なら chain 一覧）。
    pub adding: bool,
    /// 1 行目に出す、chain を掛ける先の説明（DAW なら track と音色名）。
    pub header: Line<'a>,
    /// 直近の試聴を鳴らせなかった理由。
    pub error: Option<&'a str>,
}

/// `area` の中央に overlay を重ねる。
pub fn draw(
    f: &mut Frame,
    area: Rect,
    editor: &EffectChainEditor,
    catalog: Option<&AudioEffectCatalog>,
    view: EffectChainView<'_>,
) {
    let popup = cmrt_tui_core::ui::centered_rect(70, 70, area);
    f.render_widget(Clear, popup);
    let replacing = editor.add.replace_target.is_some();
    let (title, footer) = if view.adding && replacing {
        (message::REPLACE_OVERLAY_TITLE, message::REPLACE_FOOTER)
    } else if view.adding {
        (message::ADD_OVERLAY_TITLE, message::ADD_FOOTER)
    } else {
        (message::OVERLAY_TITLE, message::FOOTER)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let error_rows = u16::from(view.error.is_some());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(error_rows),
            Constraint::Length(1),
        ])
        .split(inner);

    f.render_widget(Paragraph::new(view.header), chunks[0]);

    if view.adding {
        draw_add_panes(f, editor, catalog, chunks[1]);
    } else {
        draw_chain_list(f, editor, catalog, chunks[1]);
    }

    if let Some(error) = view.error {
        f.render_widget(
            Paragraph::new(error).style(Style::default().fg(Color::Red)),
            chunks[2],
        );
    }
    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(MONOKAI_CYAN)),
        chunks[3],
    );
}

/// chain 一覧の 1 列 List。
fn draw_chain_list(
    f: &mut Frame,
    state: &EffectChainEditor,
    catalog: Option<&AudioEffectCatalog>,
    area: Rect,
) {
    let (items, selected): (Vec<ListItem<'static>>, Option<usize>) = if state.chain.is_empty() {
        let notice = if catalog.is_none_or(|catalog| catalog.presets().is_empty()) {
            message::NO_PRESETS
        } else {
            message::EMPTY_CHAIN
        };
        (
            vec![ListItem::new(notice).style(Style::default().fg(MONOKAI_GRAY))],
            None,
        )
    } else {
        (
            state
                .chain
                .iter()
                .enumerate()
                .map(|(index, stage)| {
                    list_item(
                        format!("{}. {}", index + 1, stage_label(stage, catalog)),
                        index == state.cursor,
                    )
                })
                .collect(),
            Some(state.cursor),
        )
    };
    let mut list_state = scrolled_list_state(
        selected,
        state.chain.len(),
        usize::from(area.height),
        &state.scroll_offset,
    );
    f.render_stateful_widget(List::new(items), area, &mut list_state);
}

/// 追加・差し替え overlay の query 欄 + category / kind / list 3 pane。
fn draw_add_panes(
    f: &mut Frame,
    editor: &EffectChainEditor,
    catalog: Option<&AudioEffectCatalog>,
    area: Rect,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let add = &editor.add;

    let query_title = if add.filter_active {
        message::ADD_QUERY_TITLE_EDITING
    } else {
        message::ADD_QUERY_TITLE
    };
    let query_border = if add.filter_active {
        MONOKAI_CYAN
    } else {
        MONOKAI_FG
    };
    let query_widget = cmrt_tui_core::text_input::build_query_textarea_widget(
        &add.query_textarea,
        &add.query,
        query_title,
        message::ADD_QUERY_PLACEHOLDER,
        query_border,
    );
    f.render_widget(&query_widget, sections[0]);
    if add.filter_active {
        f.set_cursor_position(
            cmrt_tui_core::text_input::single_line_textarea_cursor_position(
                sections[0],
                &query_widget,
            ),
        );
    }

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(pane_width(&add.categories)),
            Constraint::Length(pane_width(&add.kinds)),
            Constraint::Min(1),
        ])
        .split(sections[1]);

    let category_items: Vec<ListItem<'static>> = add
        .categories
        .iter()
        .enumerate()
        .map(|(index, category)| list_item(category.clone(), index == add.category_cursor))
        .collect();
    let category_block = pane_block(" category ", add.focus, EffectAddPane::Categories);
    let mut category_state = scrolled_list_state(
        (!add.categories.is_empty()).then_some(add.category_cursor),
        add.categories.len(),
        usize::from(category_block.inner(panes[0]).height),
        add.scroll_offset(EffectAddPane::Categories),
    );
    f.render_stateful_widget(
        List::new(category_items).block(category_block),
        panes[0],
        &mut category_state,
    );

    let kind_items: Vec<ListItem<'static>> = add
        .kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| list_item(kind.clone(), index == add.kind_cursor))
        .collect();
    let kind_block = pane_block(" kind ", add.focus, EffectAddPane::Kinds);
    let mut kind_state = scrolled_list_state(
        (!add.kinds.is_empty()).then_some(add.kind_cursor),
        add.kinds.len(),
        usize::from(kind_block.inner(panes[1]).height),
        add.scroll_offset(EffectAddPane::Kinds),
    );
    f.render_stateful_widget(
        List::new(kind_items).block(kind_block),
        panes[1],
        &mut kind_state,
    );

    // 左列 plugin 名の幅は catalog 内の最長 plugin 名に揃える。
    let plugin_name_width = catalog
        .map(|catalog| {
            catalog
                .plugins()
                .iter()
                .map(|plugin| plugin.name.chars().count())
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let list_items: Vec<ListItem<'static>> = add
        .list
        .iter()
        .enumerate()
        .map(|(index, &preset_index)| {
            let text = catalog
                .and_then(|catalog| catalog.presets().get(preset_index))
                .map(|preset| {
                    let plugin_name = catalog
                        .and_then(|catalog| catalog.plugin(&preset.plugin).ok())
                        .map_or("", |plugin| plugin.name.as_str());
                    format!("{plugin_name:<plugin_name_width$}  {}", preset.name)
                })
                .unwrap_or_default();
            list_item(text, index == add.list_cursor)
        })
        .collect();
    let list_block = pane_block(" list ", add.focus, EffectAddPane::List);
    let mut list_state = scrolled_list_state(
        (!add.list.is_empty()).then_some(add.list_cursor),
        add.list.len(),
        usize::from(list_block.inner(panes[2]).height),
        add.scroll_offset(EffectAddPane::List),
    );
    f.render_stateful_widget(
        List::new(list_items).block(list_block),
        panes[2],
        &mut list_state,
    );
}

/// category / kind pane の幅。最長の名前の文字数 + 4（枠 2 + `▶ ` 2）。
fn pane_width(items: &[String]) -> u16 {
    let longest = items
        .iter()
        .map(|item| item.chars().count())
        .max()
        .unwrap_or(0);
    u16::try_from(longest + 4).unwrap_or(u16::MAX)
}

fn pane_block(title: &'static str, focus: EffectAddPane, pane: EffectAddPane) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(pane_border_color(focus, pane)))
}

fn pane_border_color(focus: EffectAddPane, pane: EffectAddPane) -> Color {
    if focus == pane {
        MONOKAI_CYAN
    } else {
        MONOKAI_FG
    }
}

/// カーソルを上下 30% の余白の内側に保つ `ListState`。余白の規則は MML overlay と同じ。
/// 表示先頭は `current` に覚えておき、余白に入るまで scroll しない。
fn scrolled_list_state(
    selected: Option<usize>,
    total: usize,
    visible_rows: usize,
    current: &Cell<usize>,
) -> ListState {
    let offset = selected.map_or(0, |cursor| {
        scroll_offset(cursor, total, visible_rows, current.get())
    });
    current.set(offset);
    ListState::default()
        .with_offset(offset)
        .with_selected(selected)
}

fn list_item(text: String, is_selected: bool) -> ListItem<'static> {
    let prefix = if is_selected { "▶ " } else { "  " };
    let style = if is_selected {
        cursor_highlight_style(Style::default().fg(MONOKAI_FG))
    } else {
        Style::default().fg(MONOKAI_FG)
    };
    ListItem::new(format!("{prefix}{text}")).style(style)
}

#[cfg(test)]
mod tests;
