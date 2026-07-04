use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::{
    super::{DawApp, DawPatchSelectPane},
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG,
};
use crate::ui_theme::cursor_highlight_style;

fn patch_items(app: &DawApp) -> Vec<ListItem<'static>> {
    app.patch_filtered
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = !app.patch_select_filter_active
                && app.patch_select_focus == DawPatchSelectPane::Patches
                && i == app.patch_cursor;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                cursor_highlight_style(Style::default().fg(MONOKAI_FG))
            } else {
                Style::default().fg(MONOKAI_FG)
            };
            ListItem::new(format!("{prefix}{item}")).style(style)
        })
        .collect()
}

fn favorite_items(app: &DawApp) -> Vec<ListItem<'static>> {
    app.patch_select_favorite_items()
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = !app.patch_select_filter_active
                && app.patch_select_focus == DawPatchSelectPane::Favorites
                && i == app.patch_favorites_cursor;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                cursor_highlight_style(Style::default().fg(MONOKAI_FG))
            } else {
                Style::default().fg(MONOKAI_FG)
            };
            ListItem::new(format!("{prefix}{item}")).style(style)
        })
        .collect()
}

fn favorite_title(app: &DawApp) -> String {
    let favorite_count = app.patch_select_favorite_items().len();
    if app.patch_favorites_query.trim().is_empty() {
        format!(" Favorite patches ({favorite_count}) ")
    } else {
        format!(
            " Favorite patches ({favorite_count}/{}) ",
            app.patch_favorite_items.len()
        )
    }
}

pub(super) fn draw_patch_select(f: &mut Frame, app: &DawApp, area: Rect) {
    let popup = crate::ui_utils::centered_rect(88, 76, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" patch select ")
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let query_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let patches_query_active =
        app.patch_select_filter_active && app.patch_select_focus == DawPatchSelectPane::Patches;
    let favorites_query_active =
        app.patch_select_filter_active && app.patch_select_focus == DawPatchSelectPane::Favorites;
    let patches_query_title = if patches_query_active {
        " Patches query (Enter=確定 / ESC=中断) "
    } else {
        " Patches query "
    };
    let favorites_query_title = if favorites_query_active {
        " Favorites query (Enter=確定 / ESC=中断) "
    } else {
        " Favorites query "
    };
    let patches_query_placeholder = if patches_query_active {
        "Patches を絞り込み"
    } else {
        "/ で Patches 絞り込み"
    };
    let favorites_query_placeholder = if favorites_query_active {
        "Favorite patches を絞り込み"
    } else {
        "/ で Favorite 絞り込み"
    };
    let patch_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_query_textarea,
        &app.patch_query,
        patches_query_title,
        patches_query_placeholder,
        MONOKAI_CYAN,
    );
    let favorites_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_favorites_query_textarea,
        &app.patch_favorites_query,
        favorites_query_title,
        favorites_query_placeholder,
        MONOKAI_CYAN,
    );
    f.render_widget(&patch_query_widget, query_panes[0]);
    f.render_widget(&favorites_query_widget, query_panes[1]);
    if app.patch_select_filter_active {
        let (area, widget) = match app.patch_select_focus {
            DawPatchSelectPane::Patches => (query_panes[0], &patch_query_widget),
            DawPatchSelectPane::Favorites => (query_panes[1], &favorites_query_widget),
        };
        f.set_cursor_position(crate::text_input::single_line_textarea_cursor_position(
            area, widget,
        ));
    }

    let patch_border = if app.patch_select_focus == DawPatchSelectPane::Patches {
        Style::default().fg(MONOKAI_CYAN)
    } else {
        Style::default().fg(MONOKAI_FG)
    };
    let favorite_border = if app.patch_select_focus == DawPatchSelectPane::Favorites {
        Style::default().fg(MONOKAI_CYAN)
    } else {
        Style::default().fg(MONOKAI_FG)
    };

    f.render_widget(
        List::new(patch_items(app)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Patches ")
                .border_style(patch_border),
        ),
        panes[0],
    );
    f.render_widget(
        List::new(favorite_items(app)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(favorite_title(app))
                .border_style(favorite_border),
        ),
        panes[1],
    );
    f.render_widget(
        Paragraph::new(if app.patch_select_filter_active {
            "?:help  Enter:検索確定  ESC:検索中断  Space:AND条件  文字:検索入力"
        } else {
            "?:help  /:現在pane検索  Enter:確定  Space:preview  ESC:閉じる  n/p/t:overlay切替  h/l・←/→:ペイン移動して preview  j/k・↑/↓:移動して preview"
        })
        .style(Style::default().fg(MONOKAI_CYAN)),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new("選択 patch で現在 track の init meas を上書き")
            .style(Style::default().fg(MONOKAI_FG)),
        chunks[3],
    );
}
