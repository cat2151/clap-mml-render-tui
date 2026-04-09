use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::{PatchPhrasePane, PatchSelectPane};
use crate::ui_theme::{MONOKAI_CYAN, MONOKAI_YELLOW};

use super::{
    status::{base_style, keybind_text, visible_list_page_size},
    Mode, TuiApp, LIST_HIGHLIGHT_SYMBOL,
};

// centered_rect() の引数は 0..=100 の割合指定。
const GUIDE_OVERLAY_WIDTH_PERCENT: u16 = 56;
const GUIDE_OVERLAY_HEIGHT_PERCENT: u16 = 36;

pub(super) fn draw_notepad_history_guide(f: &mut Frame) {
    let area = crate::ui_utils::centered_rect(
        GUIDE_OVERLAY_WIDTH_PERCENT,
        GUIDE_OVERLAY_HEIGHT_PERCENT,
        f.area(),
    );
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(vec![
            Line::from("現在の行にはpatch nameがありません。"),
            Line::from("notepad history overlayを開きます。"),
            Line::from("ENTERを押してください"),
        ])
        .style(base_style())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" お知らせ ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_YELLOW)),
        ),
        area,
    );
}

pub(super) fn draw_patch_select(
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
    crate::text_input::sync_single_line_textarea(&mut app.patch_query_textarea, &app.patch_query);
    let patch_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_query_textarea,
        search_title,
        "/ を押して絞り込み",
        MONOKAI_YELLOW,
    );
    f.render_widget(&patch_query_widget, chunks[0]);

    let patch_select_title = " 音色選択 ";
    let patch_items: Vec<ListItem> = app
        .patch_filtered
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style =
                if app.patch_select_focus == PatchSelectPane::Patches && i == app.patch_cursor {
                    base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD)
                } else {
                    base_style()
                };
            ListItem::new(Span::styled(p.clone(), style))
        })
        .collect();
    let (favorite_count, favorite_items): (usize, Vec<ListItem>) = {
        let favorites = app.patch_select_favorite_items();
        (
            favorites.len(),
            favorites
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let style = if app.patch_select_focus == PatchSelectPane::Favorites
                        && i == app.patch_favorites_cursor
                    {
                        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD)
                    } else {
                        base_style()
                    };
                    ListItem::new(Span::styled(p.clone(), style))
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

    f.render_stateful_widget(
        List::new(patch_items)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(patch_select_title)
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Favorite音色 ({}) ", favorite_count))
                    .style(base_style())
                    .border_style(favorite_border),
            )
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        panes[1],
        &mut app.patch_favorites_state,
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
    let area = crate::ui_utils::centered_rect(88, 84, f.area());
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
    app.patch_phrase_page_size = visible_list_page_size(panes[0]);

    let search_title = if app.patch_phrase_filter_active {
        " ENTERで絞り込みを決定 - patch phrase history - "
    } else {
        " ENTERでフレーズを選択 - patch phrase history - "
    };
    crate::text_input::sync_single_line_textarea(
        &mut app.patch_phrase_query_textarea,
        &app.patch_phrase_query,
    );
    let patch_phrase_query_widget = crate::text_input::build_query_textarea_widget(
        &app.patch_phrase_query_textarea,
        search_title,
        "/ を押して絞り込み (space=AND)",
        MONOKAI_YELLOW,
    );
    f.render_widget(&patch_phrase_query_widget, chunks[0]);
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
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[3],
    );
}

pub(super) fn draw_notepad_history(
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
    crate::text_input::sync_single_line_textarea(
        &mut app.notepad_query_textarea,
        &app.notepad_query,
    );
    let notepad_query_widget = crate::text_input::build_query_textarea_widget(
        &app.notepad_query_textarea,
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
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(keybind_text(&mode)).style(base_style()),
        chunks[3],
    );
}
