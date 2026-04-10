use crossterm::event::KeyCode;
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use tui_textarea::{Input, Key, TextArea};

use crate::ui_theme::{cursor_highlight_style, MONOKAI_BG, MONOKAI_FG, MONOKAI_GRAY};

fn apply_single_line_textarea_theme(textarea: &mut TextArea<'_>) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    textarea.set_cursor_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)));
}

pub(crate) fn new_single_line_textarea<'a>(text: &str) -> TextArea<'a> {
    let mut textarea = TextArea::default();
    apply_single_line_textarea_theme(&mut textarea);
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

/// Return the current single-line text value by joining all textarea lines.
///
/// This crate uses these textareas only for single-line query inputs, so joining
/// the internal lines yields the current text content.
pub(crate) fn textarea_value(textarea: &TextArea<'_>) -> String {
    textarea.lines().join("")
}

/// Build a render-only textarea widget for a query input.
///
/// The persistent `TextArea` state is cloned on purpose so each draw can attach a
/// frame-specific block title and placeholder without mutating the live input
/// state that tracks the cursor position and editing history.
pub(crate) fn build_query_textarea_widget<'a>(
    textarea: &TextArea<'a>,
    text: &str,
    title: impl Into<String>,
    placeholder: &str,
    border_color: Color,
) -> TextArea<'a> {
    let mut widget = textarea.clone();
    sync_single_line_textarea(&mut widget, text);
    apply_single_line_textarea_theme(&mut widget);
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

pub(crate) fn single_line_textarea_cursor_position(
    area: Rect,
    textarea: &TextArea<'_>,
    text: &str,
) -> Position {
    let mut textarea = textarea.clone();
    sync_single_line_textarea(&mut textarea, text);
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let (row, col) = textarea.cursor();
    let max_x = inner.x.saturating_add(inner.width.saturating_sub(1));
    let max_y = inner.y.saturating_add(inner.height.saturating_sub(1));
    Position::new(
        inner.x.saturating_add(col as u16).min(max_x),
        inner.y.saturating_add(row as u16).min(max_y),
    )
}

pub(crate) fn apply_key_code_to_textarea(textarea: &mut TextArea<'_>, key: KeyCode) -> bool {
    let Some(input) = key_code_to_input(key) else {
        return false;
    };

    let before = textarea_value(textarea);
    if !textarea.input(input) {
        return false;
    }

    textarea_value(textarea) != before
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_key_code_to_textarea_returns_false_for_cursor_only_input() {
        let mut textarea = new_single_line_textarea("pad");

        assert!(!apply_key_code_to_textarea(&mut textarea, KeyCode::Left));
        assert_eq!(textarea_value(&textarea), "pad");
    }

    #[test]
    fn apply_key_code_to_textarea_returns_true_when_text_changes() {
        let mut textarea = new_single_line_textarea("pa");

        assert!(apply_key_code_to_textarea(
            &mut textarea,
            KeyCode::Char('d')
        ));
        assert_eq!(textarea_value(&textarea), "pad");
    }
}
