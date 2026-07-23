use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
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
                "cdefgab:notes  s:transport  Shift+H:buffer  t:off/repeat/arp/auto  n:notepad  w:DAW q:quit  Ctrl+G:screens",
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
    crate::ui_utils::draw_sound_check_guide_overlay(f, screen_area, KEYBOARD_NOTE_GUIDE_MESSAGE);
}
