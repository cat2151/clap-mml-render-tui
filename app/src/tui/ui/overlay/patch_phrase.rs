use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::PatchPhrasePane;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_YELLOW};

use super::super::{
    cache_marker, mml_cache_hit,
    status::{
        base_style, keybind_text, render_status_color, render_status_text, visible_list_page_size,
    },
    Mode, TuiApp, LIST_HIGHLIGHT_SYMBOL,
};

pub(in crate::tui::ui) fn draw_patch_phrase(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let area = crate::ui_utils::centered_rect(88, 84, f.area());
    f.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    app.patch_phrase_page_size = visible_list_page_size(panes[0]);

    let search_title = if app.patch_phrase_filter_active {
        " ENTERで絞り込みを決定 - patch phrase history - "
    } else {
        " ENTERでフレーズを選択 - patch phrase history - "
    };
    let patch_phrase_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_phrase_query_textarea,
        &app.patch_phrase_query,
        search_title,
        "/ を押して絞り込み (space=AND)",
        MONOKAI_YELLOW,
    );
    f.render_widget(&patch_phrase_query_widget, chunks[0]);

    let history_entries = app.patch_phrase_history_items();
    let history_count = history_entries.len();
    let cache = app.audio_cache.lock().unwrap();
    let history_items: Vec<ListItem> = history_entries
        .into_iter()
        .enumerate()
        .map(|(i, phrase)| {
            let is_selected = !app.patch_phrase_filter_active
                && app.patch_phrase_focus == PatchPhrasePane::History
                && i == app.patch_phrase_history_cursor;
            let style = if is_selected {
                cursor_highlight_style(base_style())
            } else {
                base_style()
            };
            let preview_mml =
                app.patch_phrase_preview_mml_for_selection(PatchPhrasePane::History, i);
            let cached = preview_mml
                .as_deref()
                .is_some_and(|mml| mml_cache_hit(&cache, mml));
            let render_status = (!cached)
                .then(|| {
                    preview_mml
                        .as_deref()
                        .and_then(|mml| app.render_job_status_for_mml(mml))
                })
                .flatten();
            ListItem::new(Line::from(vec![
                Span::styled(cache_marker(cached, render_status), style),
                Span::styled(phrase, style),
            ]))
        })
        .collect();
    let favorite_entries = app.patch_phrase_favorite_items();
    let favorite_count = favorite_entries.len();
    let favorite_items: Vec<ListItem> = favorite_entries
        .into_iter()
        .enumerate()
        .map(|(i, phrase)| {
            let is_selected = !app.patch_phrase_filter_active
                && app.patch_phrase_focus == PatchPhrasePane::Favorites
                && i == app.patch_phrase_favorites_cursor;
            let style = if is_selected {
                cursor_highlight_style(base_style())
            } else {
                base_style()
            };
            let preview_mml =
                app.patch_phrase_preview_mml_for_selection(PatchPhrasePane::Favorites, i);
            let cached = preview_mml
                .as_deref()
                .is_some_and(|mml| mml_cache_hit(&cache, mml));
            let render_status = (!cached)
                .then(|| {
                    preview_mml
                        .as_deref()
                        .and_then(|mml| app.render_job_status_for_mml(mml))
                })
                .flatten();
            ListItem::new(Line::from(vec![
                Span::styled(cache_marker(cached, render_status), style),
                Span::styled(phrase, style),
            ]))
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
    let selection_status = match app.patch_phrase_focus {
        PatchPhrasePane::History => {
            super::selection_status_text(app.patch_phrase_history_cursor, history_count)
        }
        PatchPhrasePane::Favorites => {
            super::selection_status_text(app.patch_phrase_favorites_cursor, favorite_count)
        }
    };

    f.render_stateful_widget(
        List::new(history_items)
            .style(base_style())
            .highlight_style(
                if app.patch_phrase_filter_active
                    || app.patch_phrase_focus != PatchPhrasePane::History
                {
                    base_style()
                } else {
                    cursor_highlight_style(base_style())
                },
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" フレーズ選択 ")
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
            .highlight_style(
                if app.patch_phrase_filter_active
                    || app.patch_phrase_focus != PatchPhrasePane::Favorites
                {
                    base_style()
                } else {
                    cursor_highlight_style(base_style())
                },
            )
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
    if app.patch_phrase_filter_active {
        f.set_cursor_position(crate::text_input::single_line_textarea_cursor_position(
            chunks[0],
            &patch_phrase_query_widget,
        ));
    }
    let render_status_snapshot = app.render_status_snapshot();
    let render_status = render_status_text(render_status_snapshot);
    let render_color = render_status_color(render_status_snapshot);

    f.render_widget(
        Paragraph::new(format!("{status}  {selection_status}"))
            .style(base_style().fg(status_color)),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(render_status).style(base_style().fg(render_color)),
        chunks[3],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[4],
    );
}
