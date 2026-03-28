use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::{
    super::{DawApp, DawHistoryPane},
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG,
};

fn history_items(app: &DawApp) -> Vec<ListItem<'static>> {
    app.history_overlay_history_items()
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = app.history_overlay_focus == DawHistoryPane::History
                && i == app.history_overlay_history_cursor;
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
    app.history_overlay_favorite_items()
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = app.history_overlay_focus == DawHistoryPane::Favorites
                && i == app.history_overlay_favorites_cursor;
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

pub(super) fn draw_history(f: &mut Frame, app: &DawApp, area: Rect) {
    let popup = crate::ui_utils::centered_rect(88, 76, area);
    f.render_widget(Clear, popup);

    let title = match app.history_overlay_patch_name.as_deref() {
        Some(patch_name) => format!(" patch history - {patch_name} "),
        None => " history ".to_string(),
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
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let history_border = if app.history_overlay_focus == DawHistoryPane::History {
        Style::default().fg(MONOKAI_CYAN)
    } else {
        Style::default().fg(MONOKAI_FG)
    };
    let favorites_border = if app.history_overlay_focus == DawHistoryPane::Favorites {
        Style::default().fg(MONOKAI_CYAN)
    } else {
        Style::default().fg(MONOKAI_FG)
    };

    f.render_widget(
        List::new(history_items(app)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" History ")
                .border_style(history_border),
        ),
        panes[0],
    );
    f.render_widget(
        List::new(favorite_items(app)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Favorites ")
                .border_style(favorites_border),
        ),
        panes[1],
    );
    f.render_widget(
        Paragraph::new("Enter:確定  ESC:閉じる  h/l・←/→:ペイン移動  j/k:移動")
            .style(Style::default().fg(MONOKAI_CYAN)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(match app.history_overlay_patch_name.as_deref() {
            Some(_) => "現在 track の patch phrase を現在 meas に反映".to_string(),
            None => "選択行の patch を init に、phrase を現在 meas に反映".to_string(),
        })
        .style(Style::default().fg(MONOKAI_FG)),
        chunks[2],
    );
}
