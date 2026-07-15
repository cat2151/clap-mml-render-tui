use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::status::base_style;
use crate::tui::keyboard::{KeyboardConnectionPhase, KEYBOARD_NOTES};
use crate::tui::TuiApp;
use crate::ui_theme::{MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_PURPLE};

pub(super) fn draw(app: &TuiApp<'_>, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "PC key:  c   d   e   f   g   a   b",
            base_style(),
        )),
        Line::from(Span::styled(
            "Note:    C4  D4  E4  F4  G4  A4  B4",
            base_style(),
        )),
        Line::from(""),
    ];
    let active = active_notes_text(&app.keyboard_state);
    lines.push(Line::from(Span::styled(active, base_style())));
    debug_assert_eq!(KEYBOARD_NOTES.len(), 7);

    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" [KEYBOARD] keyboard mode ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        chunks[0],
    );

    let connection = app.keyboard_connection_status();
    let (state, color) = match &connection.phase {
        KeyboardConnectionPhase::Idle => ("server: idle".to_string(), MONOKAI_CYAN),
        KeyboardConnectionPhase::Connecting => ("server: connecting".to_string(), MONOKAI_PURPLE),
        KeyboardConnectionPhase::PatchSetting => {
            ("server: patch setting".to_string(), MONOKAI_PURPLE)
        }
        KeyboardConnectionPhase::Ready => ("server: ready".to_string(), MONOKAI_GREEN),
        KeyboardConnectionPhase::Error(error) => (format!("server error: {error}"), Color::Red),
    };
    let last_send = connection
        .last_send
        .map(format_send_duration)
        .unwrap_or_else(|| "-".to_string());
    let status = format!(
        "transport: {} | buffer: x{} | {state} | last send: {last_send}",
        connection.transport.label(),
        connection.buffer_multiplier
    );
    f.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new("c d e f g a b:note  h:transport  Shift+H:buffer  n:notepad  w:DAW  q:quit")
            .style(base_style()),
        chunks[2],
    );
    draw_connection_overlay(&connection.phase, f);
}

fn draw_connection_overlay(phase: &KeyboardConnectionPhase, f: &mut Frame<'_>) {
    let (title, lines, border_color, height) = match phase {
        KeyboardConnectionPhase::Ready => return,
        KeyboardConnectionPhase::Idle | KeyboardConnectionPhase::Connecting => (
            " server connection ",
            vec![
                Line::from("connecting..."),
                Line::from("c d e f g a b are unavailable until ready"),
            ],
            MONOKAI_PURPLE,
            5,
        ),
        KeyboardConnectionPhase::PatchSetting => (
            " patch setting ",
            vec![
                Line::from("patch setting..."),
                Line::from("c d e f g a b are unavailable until ready"),
            ],
            MONOKAI_PURPLE,
            5,
        ),
        KeyboardConnectionPhase::Error(error) => (
            " server connection error ",
            vec![
                Line::from(format!("server error: {error}")),
                Line::from("r:retry  n:notepad  w:DAW  q:quit"),
            ],
            Color::Red,
            7,
        ),
    };
    let width = f.area().width.saturating_sub(4).min(72);
    let area = crate::ui_utils::centered_rect_with_size(width, height, f.area());
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(base_style())
                    .border_style(base_style().fg(border_color)),
            ),
        area,
    );
}

fn format_send_duration(duration: std::time::Duration) -> String {
    let micros = duration.as_secs_f64() * 1_000_000.0;
    if micros < 1_000.0 {
        format!("{micros:.0} us")
    } else {
        format!("{:.1} ms", micros / 1_000.0)
    }
}

fn active_notes_text(state: &crate::tui::keyboard::KeyboardState) -> String {
    if state.held().is_empty() {
        return "Active: -".to_string();
    }
    let names = state
        .held()
        .iter()
        .map(|note| note.name)
        .collect::<Vec<_>>()
        .join(" ");
    format!("Active: {names}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::keyboard::KeyboardState;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_overlay(phase: KeyboardConnectionPhase) -> String {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| draw_connection_overlay(&phase, f))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer.cell((x, y)).unwrap().symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn active_notes_text_lists_every_held_note_in_press_order() {
        let mut state = KeyboardState::default();
        assert!(state.press(KEYBOARD_NOTES[0]).is_some());
        assert!(state.press(KEYBOARD_NOTES[2]).is_some());
        assert!(state.press(KEYBOARD_NOTES[4]).is_some());

        assert_eq!(active_notes_text(&state), "Active: C4 E4 G4");
    }

    #[test]
    fn active_notes_text_shows_dash_when_no_notes_are_held() {
        assert_eq!(active_notes_text(&KeyboardState::default()), "Active: -");
    }

    #[test]
    fn send_duration_uses_microseconds_then_milliseconds() {
        assert_eq!(
            format_send_duration(std::time::Duration::from_micros(42)),
            "42 us"
        );
        assert_eq!(
            format_send_duration(std::time::Duration::from_micros(12_345)),
            "12.3 ms"
        );
    }

    #[test]
    fn connecting_overlay_explains_that_notes_are_unavailable() {
        let screen = render_overlay(KeyboardConnectionPhase::Connecting);

        assert!(screen.contains("connecting..."));
        assert!(screen.contains("c d e f g a b are unavailable until ready"));
    }

    #[test]
    fn error_overlay_shows_retry_navigation() {
        let screen = render_overlay(KeyboardConnectionPhase::Error("server failed".to_string()));

        assert!(screen.contains("server error: server failed"));
        assert!(screen.contains("r:retry"));
    }

    #[test]
    fn patch_setting_overlay_remains_until_patch_is_ready() {
        let screen = render_overlay(KeyboardConnectionPhase::PatchSetting);

        assert!(screen.contains("patch setting..."));
        assert!(screen.contains("c d e f g a b are unavailable until ready"));
    }

    #[test]
    fn ready_connection_does_not_draw_an_overlay() {
        assert!(render_overlay(KeyboardConnectionPhase::Ready)
            .chars()
            .all(char::is_whitespace));
    }
}
