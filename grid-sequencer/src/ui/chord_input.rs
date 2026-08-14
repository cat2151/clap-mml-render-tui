use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_PINK},
    ui::centered_rect_with_size,
};

use crate::GridSequencerScreen;

pub(super) fn draw_overlay(frame: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let Some(input) = screen.chord_input_overlay() else {
        return;
    };
    let width = frame.area().width.saturating_sub(2).min(88);
    let height = if input.error().is_some() { 6 } else { 5 }.min(frame.area().height);
    let area = centered_rect_with_size(width, height, frame.area());
    let textarea_area = Rect::new(area.x, area.y, area.width, 3.min(area.height));
    let message_area = Rect::new(
        area.x,
        area.y.saturating_add(textarea_area.height),
        area.width,
        area.height.saturating_sub(textarea_area.height),
    );

    frame.render_widget(Clear, area);
    let value = input.value();
    let textarea = cmrt_tui_core::text_input::build_query_textarea_widget(
        input.textarea(),
        &value,
        " Fixed Chord Progression ",
        "key:G Isus4-I",
        if input.error().is_some() {
            MONOKAI_PINK
        } else {
            MONOKAI_CYAN
        },
    );
    frame.render_widget(&textarea, textarea_area);

    let mut lines = Vec::new();
    if let Some(error) = input.error() {
        lines.push(Line::styled(
            error.to_string(),
            base_style().fg(MONOKAI_PINK),
        ));
    }
    lines.push(Line::from("Enter:固定して演奏  Esc:cancel").style(base_style()));
    frame.render_widget(Paragraph::new(lines), message_area);
    frame.set_cursor_position(
        cmrt_tui_core::text_input::single_line_textarea_cursor_position(
            textarea_area,
            input.textarea(),
        ),
    );
}
