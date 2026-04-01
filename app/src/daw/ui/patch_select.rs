use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::{
    super::{DawApp, DawPatchSelectPane},
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG,
};

fn patch_items(app: &DawApp) -> Vec<ListItem<'static>> {
    app.patch_filtered
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected =
                app.patch_select_focus == DawPatchSelectPane::Patches && i == app.patch_cursor;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(MONOKAI_CYAN)
                    .add_modifier(Modifier::BOLD)
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
            let is_selected = app.patch_select_focus == DawPatchSelectPane::Favorites
                && i == app.patch_favorites_cursor;
            let prefix = if is_selected { "▶ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(MONOKAI_CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MONOKAI_FG)
            };
            ListItem::new(format!("{prefix}{item}")).style(style)
        })
        .collect()
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

    let search_title = if app.patch_select_filter_active {
        " patch select - patch name 検索入力 (Enter=確定 / ESC=中断) "
    } else {
        " patch select - / で patch name 検索 "
    };
    let search_body = if !app.patch_query.is_empty() || app.patch_select_filter_active {
        format!("/ {}", app.patch_query)
    } else {
        "/ を押して検索開始".to_string()
    };
    f.render_widget(
        Paragraph::new(search_body)
            .style(Style::default().fg(MONOKAI_FG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(search_title)
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            ),
        chunks[0],
    );

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
                .title(format!(
                    " Patches ({}/{}) ",
                    app.patch_filtered.len(),
                    app.patch_all.len()
                ))
                .border_style(patch_border),
        ),
        panes[0],
    );
    f.render_widget(
        List::new(favorite_items(app)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Favorite patches ({}) ",
                    app.patch_select_favorite_items().len()
                ))
                .border_style(favorite_border),
        ),
        panes[1],
    );
    f.render_widget(
        Paragraph::new(if app.patch_select_filter_active {
            "?:help  Enter:検索確定  ESC:検索中断  Space:AND条件  文字:検索入力"
        } else {
            "?:help  /:検索入力  Enter:確定  Space:preview  ESC:閉じる  n/p/t:overlay切替  h/l・←/→:ペイン移動してpreview  j/k・↑/↓:移動してpreview"
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
