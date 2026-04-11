use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::PatchSelectPane;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_YELLOW};

use super::super::{
    status::{
        base_style, keybind_text, parallel_render_status_color, parallel_render_status_text,
        visible_list_page_size,
    },
    Mode, TuiApp, LIST_HIGHLIGHT_SYMBOL,
};

pub(in crate::tui::ui) fn draw_patch_select(
    app: &mut TuiApp<'_>,
    f: &mut Frame,
    status: &str,
    status_color: Color,
    mode: Mode,
) {
    let area = crate::ui_utils::centered_rect(82, 70, f.area());
    f.render_widget(Clear, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    app.patch_select_page_size = visible_list_page_size(panes[0]);

    let search_title = if app.patch_select_filter_active {
        " ENTERで絞り込みを決定 - patch select - "
    } else {
        " ENTERで音色を選択 - patch select - "
    };
    let patch_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_query_textarea,
        &app.patch_query,
        search_title,
        "/ を押して絞り込み",
        MONOKAI_YELLOW,
    );
    f.render_widget(&patch_query_widget, chunks[0]);

    let patch_items: Vec<ListItem> = app
        .patch_filtered
        .iter()
        .enumerate()
        .map(|(i, patch_name)| {
            let style = if !app.patch_select_filter_active
                && app.patch_select_focus == PatchSelectPane::Patches
                && i == app.patch_cursor
            {
                cursor_highlight_style(base_style())
            } else {
                base_style()
            };
            ListItem::new(Span::styled(patch_name.clone(), style))
        })
        .collect();
    let (favorite_count, favorite_items): (usize, Vec<ListItem>) = {
        let favorites = app.patch_select_favorite_items();
        (
            favorites.len(),
            favorites
                .iter()
                .enumerate()
                .map(|(i, patch_name)| {
                    let style = if !app.patch_select_filter_active
                        && app.patch_select_focus == PatchSelectPane::Favorites
                        && i == app.patch_favorites_cursor
                    {
                        cursor_highlight_style(base_style())
                    } else {
                        base_style()
                    };
                    ListItem::new(Span::styled(patch_name.clone(), style))
                })
                .collect(),
        )
    };
    let patch_border = if app.patch_select_focus == PatchSelectPane::Patches {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
    };
    let favorite_border = if app.patch_select_focus == PatchSelectPane::Favorites {
        base_style().fg(MONOKAI_CYAN)
    } else {
        base_style()
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
                    .title(" 音色選択 ")
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
                    .title(format!(" Favorite音色 ({favorite_count}) "))
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
    let parallel_render_status = parallel_render_status_text(app.active_parallel_render_count());
    let parallel_render_color = parallel_render_status_color(app.active_parallel_render_count());

    f.render_widget(
        Paragraph::new(format!("{status}  {selection_status}"))
            .style(base_style().fg(status_color)),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(parallel_render_status).style(base_style().fg(parallel_render_color)),
        chunks[3],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[4],
    );
}
