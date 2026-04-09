use crossterm::event::KeyCode;
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};
use tui_textarea::{Input, Key, TextArea};

use crate::ui_theme::{MONOKAI_BG, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW};

pub(crate) fn new_single_line_textarea<'a>(text: &str) -> TextArea<'a> {
    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(Style::default());
    textarea.set_style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    textarea.set_cursor_style(
        Style::default()
            .fg(MONOKAI_BG)
            .bg(MONOKAI_YELLOW)
            .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK),
    );
    for ch in text.chars() {
        textarea.insert_char(ch);
    }
    textarea
}

pub(crate) fn sync_single_line_textarea<'a>(textarea: &mut TextArea<'a>, text: &str) {
    if textarea_value(textarea) != text {
        *textarea = new_single_line_textarea(text);
    }
}

pub(crate) fn textarea_value(textarea: &TextArea<'_>) -> String {
    textarea.lines().join("")
}

pub(crate) fn build_query_textarea_widget<'a>(
    textarea: &TextArea<'a>,
    title: impl Into<String>,
    placeholder: &str,
    border_color: Color,
) -> TextArea<'a> {
    let mut widget = textarea.clone();
    widget.set_style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    widget.set_cursor_line_style(Style::default());
    widget.set_cursor_style(
        Style::default()
            .fg(MONOKAI_BG)
            .bg(MONOKAI_YELLOW)
            .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK),
    );
    widget.set_placeholder_text(placeholder);
    widget.set_placeholder_style(Style::default().fg(MONOKAI_GRAY).bg(MONOKAI_BG));
    widget.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(title.into())
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .border_style(Style::default().fg(border_color)),
    );
    widget
}

pub(crate) fn apply_key_code_to_textarea(textarea: &mut TextArea<'_>, key: KeyCode) -> bool {
    key_code_to_input(key).is_some_and(|input| textarea.input(input))
}

fn key_code_to_input(key: KeyCode) -> Option<Input> {
    let key = match key {
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Tab => Key::Tab,
        KeyCode::Char(c) => Key::Char(c),
        _ => return None,
    };
    Some(Input {
        key,
        ctrl: false,
        alt: false,
        shift: false,
    })
}
