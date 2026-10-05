use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn press(screen: &mut DrumSequencerScreen, code: KeyCode) {
    screen.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn kit_changes_preserve_note_inputs_and_cursor_step() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("first".into(), Some(vec![60, 36, 60, 42]));
    assert_eq!(screen.notes(), [36, 42, 60]);
    press(&mut screen, KeyCode::Char('j'));
    press(&mut screen, KeyCode::Char('l'));
    press(&mut screen, KeyCode::Char(' '));
    assert_eq!(screen.cursor_note(), Some(42));
    assert!(screen.cell_on(42, 1));

    // 42 の行番号が変わっても、同じ note のカーソルと入力を維持する。
    screen.set_kit("second".into(), Some(vec![70, 42]));
    assert_eq!(screen.cursor_note(), Some(42));
    assert_eq!(screen.cursor_step(), 1);
    assert!(screen.cell_on(42, 1));
    assert!(!screen.cell_on(70, 1));

    // 非表示 note は消さず、現在の note が消えたら先頭を選ぶ。
    screen.set_kit("third".into(), Some(vec![80, 70]));
    assert_eq!(screen.cursor_note(), Some(70));
    assert_eq!(screen.cursor_step(), 1);
    assert!(screen.cell_on(42, 1));
    press(&mut screen, KeyCode::Enter);
    assert!(screen.cell_on(70, 1));

    screen.set_kit("first".into(), Some(vec![60, 42, 36]));
    assert_eq!(screen.cursor_note(), Some(36));
    assert_eq!(screen.cursor_step(), 1);
    assert!(screen.cell_on(42, 1));
    assert!(!screen.cell_on(36, 1));
    // 表示対象は現在の kit の一覧だけで、隠れた 70 の入力も残っている。
    assert!(!screen.notes().contains(&70));
    assert!(screen.cell_on(70, 1));
}

#[test]
fn unselected_unknown_and_empty_kits_do_not_edit_or_move() {
    let mut screen = DrumSequencerScreen::default();
    assert_eq!(screen.kit_name(), None);
    assert!(!screen.notes_known());
    for stage in 0..3 {
        if stage > 0 {
            screen.set_kit("empty".into(), if stage == 1 { None } else { Some(vec![]) });
        }
        for code in [
            KeyCode::Char('h'),
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char('l'),
            KeyCode::Char(' '),
            KeyCode::Enter,
        ] {
            press(&mut screen, code);
        }
        assert_eq!(screen.cursor_note(), None);
        assert_eq!(screen.cursor_step(), 0);
        assert!(screen.cells.iter().flatten().all(|cell| !cell));
    }
    assert!(screen.notes_known());
    assert!(screen.notes().is_empty());
}
