use std::cell::Cell;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{
    super::{
        overlays::{effect_stage_label, EffectAddPane},
        DawApp, DawMode,
    },
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY,
};
use crate::messages::effect_chain as message;
use cmrt_mml_overlay::ui::scroll_offset;
use cmrt_tui_core::theme::cursor_highlight_style;

pub(super) fn draw_effect_chain(f: &mut Frame, app: &DawApp, area: Rect) {
    let popup = cmrt_tui_core::ui::centered_rect(70, 70, area);
    f.render_widget(Clear, popup);
    let (title, footer) = if app.mode == DawMode::EffectChainAdd {
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let state = &app.overlays.effect_chain;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", crate::tracks::track_label(state.track)),
                Style::default().fg(MONOKAI_GRAY),
            ),
            Span::raw(message::INSTRUMENT_LABEL),
            Span::raw(state.instrument.as_str()),
        ])),
        chunks[0],
    );

    if app.mode == DawMode::EffectChainAdd {
        draw_add_panes(f, app, chunks[1]);
    } else {
        draw_chain_list(f, app, chunks[1]);
    }

    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(MONOKAI_CYAN)),
        chunks[2],
    );
}

/// chain 一覧（mode `EffectChain`）の 1 列 List。
fn draw_chain_list(f: &mut Frame, app: &DawApp, area: Rect) {
    let state = &app.overlays.effect_chain;
    let catalog = app.effect_plugins.catalog();
    let (items, selected): (Vec<ListItem<'static>>, Option<usize>) = if state.chain.is_empty() {
        let notice = if app.effect_preset_count() == 0 {
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
                        format!("{}. {}", index + 1, effect_stage_label(stage, catalog)),
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

/// 追加 overlay（mode `EffectChainAdd`）の query 欄 + role / list 2 pane。
fn draw_add_panes(f: &mut Frame, app: &DawApp, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let add = &app.overlays.effect_chain.add;
    let catalog = app.effect_plugins.catalog();

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
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(sections[1]);

    let role_items: Vec<ListItem<'static>> = add
        .roles
        .iter()
        .enumerate()
        .map(|(index, role)| list_item(role.clone(), index == add.role_cursor))
        .collect();
    let role_block = pane_block(" role ", add.focus, EffectAddPane::Roles);
    let mut role_state = scrolled_list_state(
        (!add.roles.is_empty()).then_some(add.role_cursor),
        add.roles.len(),
        usize::from(role_block.inner(panes[0]).height),
        add.scroll_offset(EffectAddPane::Roles),
    );
    f.render_stateful_widget(
        List::new(role_items).block(role_block),
        panes[0],
        &mut role_state,
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
        usize::from(list_block.inner(panes[1]).height),
        add.scroll_offset(EffectAddPane::List),
    );
    f.render_stateful_widget(
        List::new(list_items).block(list_block),
        panes[1],
        &mut list_state,
    );
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
