use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::status::{base_style, keybind_text, status_color, status_text, visible_list_page_size};
use crate::tui::{Mode, TuiApp};
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN};

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
                ListItem::new(Line::from(vec![
                    Span::raw("  ".repeat(node.depth)),
                    Span::raw(marker),
                    Span::raw(node.name.clone()),
                ]))
            })
            .collect::<Vec<_>>();
        frame.render_stateful_widget(
            List::new(items)
                .style(base_style())
                .highlight_style(cursor_highlight_style(base_style()))
                .highlight_symbol("▶ ")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" [LOOP BROWSER] WAV loops ")
                        .border_style(base_style().fg(MONOKAI_CYAN)),
                ),
            list_area,
            &mut app.loop_browser.list_state,
        );
    }

    let play_state = app.play_state.lock().unwrap().clone();
    frame.render_widget(
        Paragraph::new(status_text(&Mode::LoopBrowser, &play_state))
            .style(base_style().fg(status_color(&play_state))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(keybind_text(&Mode::LoopBrowser)).style(base_style()),
        chunks[2],
    );
}
