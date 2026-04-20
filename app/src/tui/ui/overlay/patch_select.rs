use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::PatchSelectPane;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_GRAY, MONOKAI_YELLOW};

use super::super::{
    cache_marker,
    status::{
        base_style, keybind_text, render_status_color, render_status_text, visible_list_page_size,
    },
    Mode, TuiApp, LIST_HIGHLIGHT_SYMBOL,
};

fn patch_cache_hit(
    app: &TuiApp<'_>,
    cache: &std::collections::HashMap<String, Vec<f32>>,
    patch_name: &str,
) -> bool {
    app.patch_select_preview_mml_for_patch_name(patch_name)
        .is_some_and(|mml| cache.contains_key(&mml))
}

pub(in crate::tui::ui) fn draw_patch_select(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let area = crate::ui_utils::centered_rect(88, 76, f.area());
    f.render_widget(Clear, area);
    let overlay_block = Block::default()
        .borders(Borders::ALL)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_YELLOW));
    let inner = overlay_block.inner(area);
    f.render_widget(overlay_block, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let patch_select_page_size = visible_list_page_size(panes[0]);
    if app.patch_select_page_size != patch_select_page_size {
        app.patch_select_page_size = patch_select_page_size;
        app.sync_patch_select_states();
    }

    let search_title = if app.patch_select_filter_active {
        " ENTERで絞り込みを決定 - patch select - "
    } else {
        " ENTERで音色を選択 - patch select - "
    };
    let active_frame_color = MONOKAI_YELLOW;
    let inactive_frame_color = MONOKAI_GRAY;
    let query_frame_color = if app.patch_select_filter_active {
        active_frame_color
    } else {
        inactive_frame_color
    };
    let patch_frame_color =
        if !app.patch_select_filter_active && app.patch_select_focus == PatchSelectPane::Patches {
            active_frame_color
        } else {
            inactive_frame_color
        };
    let favorite_frame_color = if !app.patch_select_filter_active
        && app.patch_select_focus == PatchSelectPane::Favorites
    {
        active_frame_color
    } else {
        inactive_frame_color
    };
    let query_border = base_style().fg(query_frame_color);
    let patch_border = base_style().fg(patch_frame_color);
    let favorite_border = base_style().fg(favorite_frame_color);

    let mut patch_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_query_textarea,
        &app.patch_query,
        search_title,
        "/ を押して絞り込み",
        query_frame_color,
    );
    patch_query_widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(search_title, query_border))
            .style(base_style())
            .border_style(query_border),
    );
    f.render_widget(&patch_query_widget, chunks[0]);

    let (patch_items, favorite_count, favorite_items): (Vec<ListItem>, usize, Vec<ListItem>) = {
        let cache = app.audio_cache.lock().unwrap();
        let patch_items = app
            .patch_filtered
            .iter()
            .enumerate()
            .map(|(i, patch_name)| {
                let cached = patch_cache_hit(app, &cache, patch_name);
                let style = if !app.patch_select_filter_active
                    && app.patch_select_focus == PatchSelectPane::Patches
                    && i == app.patch_cursor
                {
                    cursor_highlight_style(base_style())
                } else {
                    base_style()
                };
                let render_status = (!cached)
                    .then(|| {
                        app.patch_select_preview_mml_for_patch_name(patch_name)
                            .as_deref()
                            .and_then(|mml| app.render_job_status_for_mml(mml))
                    })
                    .flatten();
                ListItem::new(Line::from(vec![
                    Span::styled(cache_marker(cached, render_status), style),
                    Span::styled(patch_name.clone(), style),
                ]))
            })
            .collect();
        let favorites = app.patch_select_favorite_items();
        (
            patch_items,
            favorites.len(),
            favorites
                .iter()
                .enumerate()
                .map(|(i, patch_name)| {
                    let cached = patch_cache_hit(app, &cache, patch_name);
                    let style = if !app.patch_select_filter_active
                        && app.patch_select_focus == PatchSelectPane::Favorites
                        && i == app.patch_favorites_cursor
                    {
                        cursor_highlight_style(base_style())
                    } else {
                        base_style()
                    };
                    let render_status = (!cached)
                        .then(|| {
                            app.patch_select_preview_mml_for_patch_name(patch_name)
                                .as_deref()
                                .and_then(|mml| app.render_job_status_for_mml(mml))
                        })
                        .flatten();
                    ListItem::new(Line::from(vec![
                        Span::styled(cache_marker(cached, render_status), style),
                        Span::styled(patch_name.clone(), style),
                    ]))
                })
                .collect(),
        )
    };
    let selection_status = match app.patch_select_focus {
        PatchSelectPane::Patches => {
            super::selection_status_text(app.patch_cursor, app.patch_filtered.len())
        }
        PatchSelectPane::Favorites => {
            super::selection_status_text(app.patch_favorites_cursor, favorite_count)
        }
    };

    f.render_stateful_widget(
        List::new(patch_items)
            .style(base_style())
            .highlight_style(
                if app.patch_select_filter_active
                    || app.patch_select_focus != PatchSelectPane::Patches
                {
                    base_style()
                } else {
                    cursor_highlight_style(base_style())
                },
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(" 音色選択 ", patch_border))
                    .style(base_style())
                    .border_style(patch_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[0],
        &mut app.patch_list_state,
    );
    f.render_stateful_widget(
        List::new(favorite_items)
            .style(base_style())
            .highlight_style(
                if app.patch_select_filter_active
                    || app.patch_select_focus != PatchSelectPane::Favorites
                {
                    base_style()
                } else {
                    cursor_highlight_style(base_style())
                },
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(" Favorite音色 ({favorite_count}) "),
                        favorite_border,
                    ))
                    .style(base_style())
                    .border_style(favorite_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[1],
        &mut app.patch_favorites_state,
    );
    if app.patch_select_filter_active {
        f.set_cursor_position(crate::text_input::single_line_textarea_cursor_position(
            chunks[0],
            &patch_query_widget,
        ));
    }
    let render_status_snapshot = app.render_status_snapshot();
    let render_status = render_status_text(render_status_snapshot);
    let render_color = render_status_color(render_status_snapshot);

    f.render_widget(
        Paragraph::new(format!(
            "{status}  {selection_status}  sort:{}",
            app.patch_select_sort_order.status_label()
        ))
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
