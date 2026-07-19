use ratatui::{
    layout::Rect,
    style::Color,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

use super::base_style;
use crate::tui::keyboard::KeyboardMmlInput;
use crate::ui_theme::MONOKAI_GREEN;

pub(super) fn draw_mml_input_overlay(
    input: &KeyboardMmlInput<'_>,
    f: &mut Frame<'_>,
    keyboard_area: Rect,
) {
    if !input.is_active() {
        return;
    }
    let has_error = input.error().is_some();
    let width = keyboard_area.width.saturating_sub(2).min(72);
    let height = if has_error { 6 } else { 5 }.min(keyboard_area.height);
    let area = crate::ui_utils::centered_rect_with_size(width, height, keyboard_area);
    let textarea_area = Rect::new(area.x, area.y, area.width, 3.min(area.height));
    let message_area = Rect::new(
        area.x,
        area.y.saturating_add(textarea_area.height),
        area.width,
        area.height.saturating_sub(textarea_area.height),
    );

    f.render_widget(Clear, area);
    let value = input.value();
    let textarea = crate::text_input::build_query_textarea_widget(
        input.textarea(),
        &value,
        " MML notes ",
        "例: o4ceg",
        MONOKAI_GREEN,
    );
    f.render_widget(&textarea, textarea_area);

    let mut lines = Vec::new();
    if let Some(error) = input.error() {
        lines.push(Line::styled(error, base_style().fg(Color::Red)));
    }
    lines.push(Line::from("Enter:確定  Esc:cancel").style(base_style()));
    f.render_widget(Paragraph::new(lines), message_area);
    f.set_cursor_position(crate::text_input::single_line_textarea_cursor_position(
        textarea_area,
        input.textarea(),
    ));
}
