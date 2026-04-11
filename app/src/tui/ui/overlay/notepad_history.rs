use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::PatchPhrasePane;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_YELLOW};

use super::super::{
    status::{base_style, keybind_text, visible_list_page_size},
    Mode, TuiApp, LIST_HIGHLIGHT_SYMBOL,
};

pub(in crate::tui::ui) fn draw_notepad_history(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let area = crate::ui_utils::centered_rect(88, 76, f.area());
    f.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    app.notepad_history_page_size = visible_list_page_size(panes[0]);

    let search_title = if app.notepad_filter_active {
        " ENTERで絞り込みを決定 - notepad history - "
    } else {
        " ENTERで音色とフレーズを選択 - notepad history - "
    };
    let notepad_query_widget = crate::text_input::build_query_textarea_widget(
        &app.notepad_query_textarea,
        &app.notepad_query,
        search_title,
        "/ を押して絞り込み (space=AND)",
        MONOKAI_YELLOW,
    );
    f.render_widget(&notepad_query_widget, chunks[0]);

    let history_items: Vec<ListItem> = app
        .notepad_history_items()
        .into_iter()
        .enumerate()
        .map(|(i, mml)| {
            let is_selected = !app.notepad_filter_active
                && app.notepad_focus == PatchPhrasePane::History
                && i == app.notepad_history_cursor;
            let style = if is_selected {
                cursor_highlight_style(base_style())
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
            let is_selected = !app.notepad_filter_active
                && app.notepad_focus == PatchPhrasePane::Favorites
                && i == app.notepad_favorites_cursor;
            let style = if is_selected {
                cursor_highlight_style(base_style())
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
    let history_highlight_style =
        if app.notepad_filter_active || app.notepad_focus != PatchPhrasePane::History {
            base_style()
        } else {
            cursor_highlight_style(base_style())
        };
    let favorites_highlight_style =
        if app.notepad_filter_active || app.notepad_focus != PatchPhrasePane::Favorites {
            base_style()
        } else {
            cursor_highlight_style(base_style())
        };

    f.render_stateful_widget(
        List::new(history_items)
            .style(base_style())
            .highlight_style(history_highlight_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 音色 & フレーズ選択 ")
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
            .highlight_style(favorites_highlight_style)
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
    if app.notepad_filter_active {
        f.set_cursor_position(crate::text_input::single_line_textarea_cursor_position(
            chunks[0],
            &notepad_query_widget,
        ));
    }

    f.render_widget(
        Paragraph::new(status.to_string()).style(base_style().fg(status_color)),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[3],
    );
}
