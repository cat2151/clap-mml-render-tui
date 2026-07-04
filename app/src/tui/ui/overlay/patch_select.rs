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
    disk_hashes: &std::collections::HashSet<u64>,
    patch_name: &str,
) -> bool {
    app.patch_select_preview_mml_for_patch_name(patch_name)
        .is_some_and(|mml| {
            cache.contains_key(&mml)
                || disk_hashes.contains(&crate::history::daw_cache_mml_hash(&mml))
        })
}

fn favorite_title(app: &TuiApp<'_>, favorite_count: usize) -> String {
    if app.patch_favorites_query.trim().is_empty() {
        format!(" Favorite音色 ({favorite_count}) ")
    } else {
        format!(
            " Favorite音色 ({favorite_count}/{}) ",
            app.patch_favorite_items.len()
        )
    }
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
    let query_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);
    let patch_select_page_size = visible_list_page_size(panes[0]);
    if app.patch_select_page_size != patch_select_page_size {
        app.patch_select_page_size = patch_select_page_size;
        app.sync_patch_select_states();
    }

    let active_frame_color = MONOKAI_YELLOW;
    let inactive_frame_color = MONOKAI_GRAY;
    let patches_query_active =
        app.patch_select_filter_active && app.patch_select_focus == PatchSelectPane::Patches;
    let favorites_query_active =
        app.patch_select_filter_active && app.patch_select_focus == PatchSelectPane::Favorites;
    let patches_query_frame_color = if patches_query_active {
        active_frame_color
    } else {
        inactive_frame_color
    };
    let favorites_query_frame_color = if favorites_query_active {
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
    let patches_query_border = base_style().fg(patches_query_frame_color);
    let favorites_query_border = base_style().fg(favorites_query_frame_color);
    let patch_border = base_style().fg(patch_frame_color);
    let favorite_border = base_style().fg(favorite_frame_color);
    let patches_query_title = if patches_query_active {
        " Patches query (Enter=決定) "
    } else {
        " Patches query "
    };
    let favorites_query_title = if favorites_query_active {
        " Favorites query (Enter=決定) "
    } else {
        " Favorites query "
    };
    let patches_query_placeholder = if patches_query_active {
        "Patches を絞り込み"
    } else {
        "/ で Patches 絞り込み"
    };
    let favorites_query_placeholder = if favorites_query_active {
        "Favorite音色を絞り込み"
    } else {
        "/ で Favorite 絞り込み"
    };

    let mut patch_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_query_textarea,
        &app.patch_query,
        patches_query_title,
        patches_query_placeholder,
        patches_query_frame_color,
    );
    patch_query_widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(patches_query_title, patches_query_border))
            .style(base_style())
            .border_style(patches_query_border),
    );
    let mut favorites_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_favorites_query_textarea,
        &app.patch_favorites_query,
        favorites_query_title,
        favorites_query_placeholder,
        favorites_query_frame_color,
    );
    favorites_query_widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(favorites_query_title, favorites_query_border))
            .style(base_style())
            .border_style(favorites_query_border),
    );
    f.render_widget(&patch_query_widget, query_panes[0]);
    f.render_widget(&favorites_query_widget, query_panes[1]);

    let (patch_items, favorite_count, favorite_items): (Vec<ListItem>, usize, Vec<ListItem>) = {
        let cache = app.audio_cache.lock().unwrap();
        let disk_hashes = app.known_disk_cache_hashes.lock().unwrap();
        let patch_items = app
            .patch_filtered
            .iter()
            .enumerate()
            .map(|(i, patch_name)| {
                let cached = patch_cache_hit(app, &cache, &disk_hashes, patch_name);
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
                    let cached = patch_cache_hit(app, &cache, &disk_hashes, patch_name);
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
                        favorite_title(app, favorite_count),
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
        let (area, widget) = match app.patch_select_focus {
            PatchSelectPane::Patches => (query_panes[0], &patch_query_widget),
            PatchSelectPane::Favorites => (query_panes[1], &favorites_query_widget),
        };
        f.set_cursor_position(crate::text_input::single_line_textarea_cursor_position(
            area, widget,
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
