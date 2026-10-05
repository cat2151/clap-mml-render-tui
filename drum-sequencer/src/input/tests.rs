use super::*;

fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
}

#[test]
fn hjkl_reaches_every_note_and_step_and_stops_at_edges() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("full range".into(), Some((0..128).collect()));
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Char('h'), KeyEventKind::Press));
        screen.handle_key_event(key(KeyCode::Char('k'), KeyEventKind::Press));
    }
    assert_eq!(screen.cursor_note(), Some(0));
    assert_eq!(screen.cursor_step(), 0);

    for note in 0..128 {
        assert_eq!(screen.cursor_note(), Some(note));
        for step in 0..DRUM_STEPS {
            assert_eq!(screen.cursor_step(), step);
            screen.handle_key_event(key(KeyCode::Char('l'), KeyEventKind::Repeat));
        }
        assert_eq!(screen.cursor_step(), 15);
        for _ in 0..20 {
            screen.handle_key_event(key(KeyCode::Char('h'), KeyEventKind::Repeat));
        }
        screen.handle_key_event(key(KeyCode::Char('j'), KeyEventKind::Repeat));
    }
    assert_eq!(screen.cursor_note(), Some(127));
    for _ in 0..130 {
        screen.handle_key_event(key(KeyCode::Char('k'), KeyEventKind::Repeat));
    }
    assert_eq!(screen.cursor_note(), Some(0));
}

#[test]
fn space_and_enter_toggle_once_per_press_and_ignore_repeat_and_release() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".into(), Some(vec![36]));
    for code in [KeyCode::Char(' '), KeyCode::Enter] {
        screen.handle_key_event(key(code, KeyEventKind::Press));
        assert!(screen.cell_on(36, 0));
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            screen.handle_key_event(key(code, kind));
            assert!(screen.cell_on(36, 0));
        }
        screen.handle_key_event(key(code, KeyEventKind::Press));
        assert!(!screen.cell_on(36, 0));
        screen.handle_key_event(key(code, KeyEventKind::Release));
        assert!(!screen.cell_on(36, 0));
    }
    screen.handle_key_event(key(KeyCode::Char('l'), KeyEventKind::Release));
    assert_eq!(screen.cursor_step(), 0);
}
