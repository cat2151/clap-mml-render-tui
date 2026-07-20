use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::status::{base_style, keybind_text, status_color, status_text, visible_list_page_size};
use crate::tui::{Mode, TuiApp};
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_YELLOW};

pub(super) fn draw(app: &mut TuiApp<'_>, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let list_area = chunks[0];
    app.loop_browser.page_size = visible_list_page_size(list_area);

    if let Some(error) = &app.loop_browser.error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .wrap(Wrap { trim: false })
                .style(base_style().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" [LOOP BROWSER] WAV loops ")
                        .border_style(base_style().fg(MONOKAI_CYAN)),
                ),
            list_area,
        );
    } else {
        let items = app
            .loop_browser
            .visible
            .iter()
            .map(|node| {
                let marker = if node.is_wav {
                    "♪ "
                } else if node.expanded {
                    "▾ "
                } else {
                    "▸ "
                };
                let favorite = if !node.is_wav && node.favorite {
                    "★ "
                } else {
                    ""
                };
                let category = node
                    .category
                    .as_ref()
                    .map(|category| format!(" [{category}]"))
                    .unwrap_or_default();
                ListItem::new(Line::from(vec![
                    Span::raw("  ".repeat(node.depth)),
                    Span::raw(marker),
                    Span::styled(favorite, base_style().fg(MONOKAI_YELLOW)),
                    Span::raw(node.name.clone()),
                    Span::styled(category, base_style().fg(MONOKAI_GREEN)),
                ]))
            })
            .collect::<Vec<_>>();
        let title = if app.loop_browser.favorites_only {
            " [LOOP BROWSER] Favorite dirs "
        } else {
            " [LOOP BROWSER] WAV loops "
        };
        frame.render_stateful_widget(
            List::new(items)
                .style(base_style())
                .highlight_style(cursor_highlight_style(base_style()))
                .highlight_symbol("▶ ")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(base_style().fg(MONOKAI_CYAN)),
                ),
            list_area,
            &mut app.loop_browser.list_state,
        );
    }

    let play_state = app.play_state.lock().unwrap().clone();
    let (status, color) = if let Some(error) = &app.loop_browser.metadata_error {
        (error.clone(), Color::Red)
    } else {
        (
            status_text(&Mode::LoopBrowser, &play_state),
            status_color(&play_state),
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(keybind_text(&Mode::LoopBrowser)).style(base_style()),
        chunks[2],
    );

    if app.loop_browser.category_overlay.is_some() {
        draw_category_overlay(app, frame);
    }
    if let Some(notice) = app
        .loop_browser
        .active_notice()
        .map(|notice| notice.text.clone())
    {
        draw_notice(frame, &notice);
    }
}

fn draw_category_overlay(app: &TuiApp<'_>, frame: &mut Frame<'_>) {
    let current = app.loop_browser.category_overlay_current();
    let lines = app
        .loop_browser
        .category_keys
        .iter()
        .map(|(key, category)| {
            let marker = if current == Some(category.as_str()) {
                "●"
            } else {
                " "
            };
            Line::from(format!(" {marker} {key}: {category} "))
        })
        .collect::<Vec<_>>();
    let area = crate::ui_utils::centered_text_block_rect(
        frame.area(),
        " dirカテゴリ (Esc:キャンセル) ",
        &lines,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" dirカテゴリ (Esc:キャンセル) ")
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn draw_notice(frame: &mut Frame<'_>, message: &str) {
    let lines = vec![Line::from(Span::styled(
        message.to_string(),
        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
    ))];
    let area = crate::ui_utils::centered_text_block_rect(frame.area(), " お知らせ ", &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" お知らせ ")
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}
