//! `h` / `l` はカーソルを動かし、1 音モードに依らずカーソル列の音を求める。

use super::{key, screen_with_mml};
use crate::{GuitarArticulationAction, GuitarArticulationScreen, Take};
use crossterm::event::KeyCode;

#[test]
fn h_and_l_move_the_cursor_and_play_the_cursor_note_even_without_note_preview() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    assert!(!screen.note_preview());
    let note = |column| GuitarArticulationAction::PlayNote {
        take: Take::Converted,
        column,
    };

    assert_eq!(screen.handle_key_event(key(KeyCode::Char('h'))), note(0));
    assert_eq!(screen.cursor(), 0, "左端で止まり、その列を鳴らす");
    for expected in [1, 2, 2] {
        assert_eq!(
            screen.handle_key_event(key(KeyCode::Char('l'))),
            note(expected)
        );
        assert_eq!(screen.cursor(), expected);
    }
    screen.handle_key_event(key(KeyCode::Char('n')));
    assert_eq!(screen.handle_key_event(key(KeyCode::Left)), note(1));
}

#[test]
fn h_and_l_without_mml_play_nothing() {
    let mut screen = GuitarArticulationScreen::default();
    for code in [KeyCode::Char('h'), KeyCode::Char('l')] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            GuitarArticulationAction::Continue
        );
    }
    assert!(screen.error.is_none());
}
