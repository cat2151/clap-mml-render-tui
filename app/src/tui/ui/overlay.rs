use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier},
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::PatchPhrasePane;
use crate::ui_theme::{MONOKAI_CYAN, MONOKAI_YELLOW};

use super::{
    status::{base_style, keybind_text, visible_list_page_size},
    Mode, TuiApp, LIST_HIGHLIGHT_SYMBOL,
};

pub(super) fn draw_patch_select(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let area = crate::ui_utils::centered_rect(70, 70, f.area());
    f.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    app.patch_select_page_size = visible_list_page_size(chunks[1]);

    f.render_widget(
        Paragraph::new(format!("> {}", app.patch_query))
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 音色選択 - 検索 (space=AND) ")
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_YELLOW)),
            ),
        chunks[0],
    );

    let count_title = format!(
        " パッチ ({}/{}) ",
        app.patch_filtered.len(),
        app.patch_all.len()
    );
    let patch_items: Vec<ListItem> = app
        .patch_filtered
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.patch_cursor {
                base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(p.clone(), style))
        })
        .collect();

    f.render_stateful_widget(
        List::new(patch_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(count_title)
                    .style(base_style())
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        chunks[1],
        &mut app.patch_list_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(base_style().fg(status_color)),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[3],
    );
}

pub(super) fn draw_patch_phrase(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);
    app.patch_phrase_page_size = visible_list_page_size(panes[0]);

    let patch_name = app.patch_phrase_name.as_deref().unwrap_or("(unknown)");
    let history_items: Vec<ListItem> = app
        .patch_phrase_history_items()
        .into_iter()
        .enumerate()
        .map(|(i, phrase)| {
            let is_selected = app.patch_phrase_focus == PatchPhrasePane::History
                && i == app.patch_phrase_history_cursor;
            let style = if is_selected {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(phrase, style))
        })
        .collect();
    let favorite_items: Vec<ListItem> = app
        .patch_phrase_favorite_items()
        .into_iter()
        .enumerate()
        .map(|(i, phrase)| {
            let is_selected = app.patch_phrase_focus == PatchPhrasePane::Favorites
                && i == app.patch_phrase_favorites_cursor;
            let style = if is_selected {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(phrase, style))
        })
        .collect();

    let history_border = if app.patch_phrase_focus == PatchPhrasePane::History {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };
    let favorites_border = if app.patch_phrase_focus == PatchPhrasePane::Favorites {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };

    f.render_stateful_widget(
        List::new(history_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" History - {patch_name} "))
                    .style(base_style())
                    .border_style(history_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[0],
        &mut app.patch_phrase_history_state,
    );
    f.render_stateful_widget(
        List::new(favorite_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Favorites ")
                    .style(base_style())
                    .border_style(favorites_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[1],
        &mut app.patch_phrase_favorites_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[2],
    );
}

pub(super) fn draw_notepad_history(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let area = crate::ui_utils::centered_rect(80, 70, f.area());
    f.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);
    app.notepad_history_page_size = visible_list_page_size(panes[0]);

    let history_items: Vec<ListItem> = app
        .notepad_history_items()
        .into_iter()
        .enumerate()
        .map(|(i, mml)| {
            let is_selected =
                app.notepad_focus == PatchPhrasePane::History && i == app.notepad_history_cursor;
            let style = if is_selected {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(mml, style))
        })
        .collect();
    let favorite_items: Vec<ListItem> = app
        .notepad_favorite_items()
        .into_iter()
        .enumerate()
        .map(|(i, mml)| {
            let is_selected = app.notepad_focus == PatchPhrasePane::Favorites
                && i == app.notepad_favorites_cursor;
            let style = if is_selected {
                base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
            } else {
                base_style()
            };
            ListItem::new(Span::styled(mml, style))
        })
        .collect();

    let history_border = if app.notepad_focus == PatchPhrasePane::History {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };
    let favorites_border = if app.notepad_focus == PatchPhrasePane::Favorites {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };
    let favorites_title =
        if app.notepad_focus == PatchPhrasePane::Favorites && app.notepad_pending_delete {
            " Favorites (dd:削除) "
        } else {
            " Favorites "
        };

    f.render_stateful_widget(
        List::new(history_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" History ")
                    .style(base_style())
                    .border_style(history_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[0],
        &mut app.notepad_history_state,
    );
    f.render_stateful_widget(
        List::new(favorite_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(favorites_title)
                    .style(base_style())
                    .border_style(favorites_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[1],
        &mut app.notepad_favorites_state,
    );

    f.render_widget(
        Paragraph::new(status.to_string()).style(base_style().fg(status_color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[2],
    );
}
