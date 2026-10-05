use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use super::*;

fn press(screen: &mut DrumSequencerScreen, code: KeyCode, count: usize) {
    for _ in 0..count {
        screen.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

fn render(screen: &mut DrumSequencerScreen, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(screen, frame)).unwrap();
    terminal.backend().buffer().clone()
}

fn rows(buffer: &Buffer, area: Rect) -> Vec<String> {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect()
        })
        .collect()
}

#[test]
fn vertical_scroll_keeps_the_cursor_visible_down_up_and_after_resize() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("full range".into(), Some((0..128).collect()));
    press(&mut screen, KeyCode::Char('j'), 100);
    for (up, height, note) in [(0, 12, 100), (8, 12, 92), (0, 30, 92)] {
        press(&mut screen, KeyCode::Char('k'), up);
        let buffer = render(&mut screen, 80, height);
        let matrix = rows(&buffer, layout_for(buffer.area).matrix);
        let selected: Vec<_> = matrix.iter().filter(|row| row.contains("[.]")).collect();
        assert_eq!(selected.len(), 1, "{matrix:?}");
        assert!(selected[0].starts_with(&format!("{note:>3} ")));
    }
}

#[test]
fn matrix_renders_all_sixteen_steps_and_retains_them_in_narrow_terminals() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]));
    press(&mut screen, KeyCode::Char('l'), 15);
    press(&mut screen, KeyCode::Enter, 1);
    let buffer = render(&mut screen, 80, 12);
    let layout = layout_for(buffer.area);
    let headings = rows(&buffer, layout.steps);
    for step in 1..=DRUM_STEPS {
        assert!(headings[0].contains(&format!("{step:^STEP_WIDTH$}")));
    }
    let matrix = rows(&buffer, layout.matrix);
    assert!(matrix[0].starts_with(" 36 "));
    assert_eq!(matrix[0].chars().filter(|ch| *ch == '.').count(), 15);
    assert_eq!(matrix[0].find("[x]"), Some(NOTE_WIDTH + 15 * STEP_WIDTH));

    for (width, height) in [(0, 0), (1, 1), (8, 3), (20, 10), (53, 12)] {
        render(&mut screen, width, height);
        assert_eq!(screen.notes(), [36]);
        assert_eq!(screen.cursor_step(), 15);
        assert!(screen.cell_on(36, 15));
    }
    let buffer = render(&mut screen, 80, 12);
    assert!(rows(&buffer, layout_for(buffer.area).matrix)[0].contains("[x]"));
}

#[test]
fn empty_views_explain_unselected_missing_catalog_and_known_empty_notes() {
    let mut screen = DrumSequencerScreen::default();
    let buffer = render(&mut screen, 80, 14);
    let text = rows(&buffer, layout_for(buffer.area).matrix).concat();
    assert!(text.contains("t"));
    assert!(!text.contains("[.]"));

    screen.set_kit("old catalog".into(), None);
    let buffer = render(&mut screen, 80, 14);
    let text: String = rows(&buffer, layout_for(buffer.area).matrix)
        .concat()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(text.contains("cmrtbuild-patch-catalog-cache"));

    screen.set_kit("empty".into(), Some(vec![]));
    let buffer = render(&mut screen, 80, 14);
    let text = rows(&buffer, layout_for(buffer.area).matrix).concat();
    assert!(!text.contains("build-patch-catalog-cache"));
    assert!(!text.contains("[.]"));
}
