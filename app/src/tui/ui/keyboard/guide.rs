use ratatui::{
    layout::{Alignment, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::base_style;
use crate::tui::keyboard::guide::{KeyboardNoteGuidePresentation, KEYBOARD_NOTE_GUIDE_MESSAGE};
use crate::ui_theme::{MONOKAI_CYAN, MONOKAI_YELLOW};

pub(super) fn keyboard_help_lines(
    presentation: KeyboardNoteGuidePresentation,
    navigation_count: Option<usize>,
) -> Vec<Line<'static>> {
    if presentation == KeyboardNoteGuidePresentation::Footer {
        return vec![
            Line::default(),
            Line::from(Span::styled(
                KEYBOARD_NOTE_GUIDE_MESSAGE,
                base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
            )),
            Line::default(),
        ];
    }

    match navigation_count {
        Some(count) => vec![
            Line::from(vec![
                Span::styled(
                    format!("Count: {count}_"),
                    base_style()
                        .fg(MONOKAI_YELLOW)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    "0-9 または h/j/k/l/Ctrl+u/Ctrl+d を押してください",
                    base_style()
                        .fg(MONOKAI_CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::default(),
            Line::default(),
        ],
        None => vec![
            Line::from(concat!(
                "k/j/Up/Down:patch -/+1  Ctrl+u/d/PgUp/PgDn:patch -/+10  ",
                "h/l/Home/End:cat -/+1 r:random"
            )),
            Line::from(
                "cdefgab:notes  s:transport  Shift+H:buffer  t:off/repeat/arp/auto  n:notepad  w:DAW q:quit",
            ),
            Line::from(
                "i:MML notes  v:velocity  m:mod(CC1)  p:pitch bend  x:CC#  z:CC value  Shift+Z:CC cycle",
            ),
        ],
    }
}

pub(super) fn draw_note_guide_overlay(
    presentation: KeyboardNoteGuidePresentation,
    f: &mut Frame<'_>,
    screen_area: Rect,
) {
    if presentation != KeyboardNoteGuidePresentation::Overlay {
        return;
    }

    let width = screen_area.width.saturating_sub(2).min(72);
    let height = 5.min(screen_area.height);
    let area = crate::ui_utils::centered_rect_with_size(width, height, screen_area);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            KEYBOARD_NOTE_GUIDE_MESSAGE,
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(base_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 音出し確認 ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}
