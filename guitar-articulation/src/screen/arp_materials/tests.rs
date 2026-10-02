use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::ArpRow;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn char_key(ch: char) -> KeyEvent {
    key(KeyCode::Char(ch))
}

/// `i` → 文字 → `Enter` で `mml` を確定し、素材リストを持たせて `z` で overlay を開いた画面。
/// param pane は素材の行を選んでいる。
fn overlay_screen(mml: &str, materials: &[&str]) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default()
        .with_arp_materials(materials.iter().map(|m| m.to_string()).collect());
    screen.handle_key_event(char_key('i'));
    for ch in mml.chars() {
        screen.handle_key_event(char_key(ch));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.mml(), mml);
    screen.handle_key_event(char_key('z'));
    assert!(screen.arp_overlay_open());
    assert_eq!(
        screen.arp_rows()[screen.arp_row_index().unwrap()],
        ArpRow::Material
    );
    screen
}

fn type_text(screen: &mut GuitarArticulationScreen, text: &str) {
    for ch in text.chars() {
        screen.handle_key_event(char_key(ch));
    }
}

#[test]
fn h_and_l_on_the_material_row_do_nothing_with_an_empty_list() {
    let mut screen = overlay_screen("l16cdef", &[]);
    let history = screen.history().clone();
    for ch in ['l', 'h'] {
        assert_eq!(
            screen.handle_key_event(char_key(ch)),
            GuitarArticulationAction::Continue
        );
        assert_eq!(screen.mml(), "l16cdef");
        assert_eq!(*screen.history(), history);
    }
}

#[test]
fn h_and_l_on_the_material_row_cycle_through_the_list_and_wrap() {
    let mut screen = overlay_screen("l16cdef", &["C", "Am7", "l16cdef"]);
    assert_eq!(screen.arp_material_position(), Some(2));

    assert_eq!(
        screen.handle_key_event(char_key('l')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.arp_material(), "C");
    assert!(screen.material_from_chord());
    screen.handle_key_event(char_key('l'));
    assert_eq!(screen.arp_material(), "Am7");
    assert_eq!(screen.mml(), "l16cdef", "MML 欄は変えない");
    screen.handle_key_event(char_key('h'));
    screen.handle_key_event(char_key('h'));
    assert_eq!(screen.arp_material(), "l16cdef");
    assert!(!screen.material_from_chord());
    assert!(screen.arp_overlay_open());
}

#[test]
fn a_material_outside_the_list_goes_to_the_first_or_the_last() {
    let mut next = overlay_screen("l8cde", &["C", "Am7", "l16cdef"]);
    assert_eq!(next.arp_material_position(), None);
    next.handle_key_event(char_key('l'));
    assert_eq!(next.arp_material(), "C");

    let mut previous = overlay_screen("l8cde", &["C", "Am7", "l16cdef"]);
    previous.handle_key_event(char_key('h'));
    assert_eq!(previous.arp_material(), "l16cdef");
}

#[test]
fn a_material_chosen_with_l_is_dropped_from_the_main_screen_on_close() {
    let mut screen = overlay_screen("l16cdef", &["Am7"]);
    let history = screen.history().clone();

    assert_eq!(
        screen.handle_key_event(char_key('l')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.arp_material(), "Am7");
    assert_eq!(screen.sounding_mml(), "Am7");
    assert_eq!(
        screen.events(Take::Plain),
        crate::material_performance("Am7", screen.applied_arp())
            .unwrap()
            .events
            .as_slice()
    );
    screen.handle_key_event(key(KeyCode::Esc));

    assert_eq!(screen.mml(), "l16cdef");
    assert_eq!(screen.sounding_mml(), "l16cdef");
    assert_eq!(
        screen.events(Take::Plain),
        crate::performance_events("l16cdef", None)
            .unwrap()
            .as_slice()
    );
    assert_eq!(*screen.history(), history);
}

#[test]
fn a_material_chosen_in_the_material_pane_is_dropped_from_the_main_screen_on_close() {
    let mut screen = overlay_screen("l16cdef", &["C", "Am7"]);
    screen.handle_key_event(key(KeyCode::Tab));
    screen.handle_key_event(key(KeyCode::Up));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Tab)),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.arp_material(), "C");

    screen.handle_key_event(key(KeyCode::Esc));

    assert_eq!(screen.mml(), "l16cdef");
    assert_eq!(
        screen.events(Take::Plain),
        crate::performance_events("l16cdef", None)
            .unwrap()
            .as_slice()
    );
}

#[test]
fn reopening_the_overlay_starts_with_the_material_chosen_before() {
    let mut screen = overlay_screen("l16cdef", &["Am7"]);
    screen.handle_key_event(char_key('l'));
    screen.handle_key_event(key(KeyCode::Esc));

    screen.handle_key_event(char_key('z'));

    assert_eq!(screen.arp_material(), "Am7");
    assert_eq!(screen.arp_material_position(), Some(0));
    assert_eq!(
        screen.events(Take::Plain),
        crate::material_performance("Am7", screen.applied_arp())
            .unwrap()
            .events
            .as_slice()
    );
}

#[test]
fn tab_opens_the_material_pane_on_the_current_line_and_letters_are_typed() {
    let mut screen = overlay_screen("Am7", &["C", "Am7", "G"]);
    screen.handle_key_event(key(KeyCode::Tab));
    let input = screen.arp_materials_input().expect("素材 pane を編集中");
    assert_eq!(input.lines(), ["C", "Am7", "G"]);
    assert_eq!(input.cursor(), (1, 3));

    // param pane のキーも文字として入る。
    type_text(&mut screen, "jl");
    assert_eq!(screen.arp_materials_input().unwrap().lines()[1], "Am7jl");
    assert_eq!(screen.mml(), "Am7");
    assert!(!screen.take_unsaved_settings(), "確定するまで変えない");
}

#[test]
fn leaving_the_material_pane_saves_the_list_and_plays_the_cursor_line() {
    for leave in [KeyCode::Tab, KeyCode::Esc] {
        let mut screen = overlay_screen("l16cdef", &[]);
        screen.handle_key_event(key(KeyCode::Tab));
        type_text(&mut screen, "C");
        screen.handle_key_event(key(KeyCode::Enter));
        type_text(&mut screen, " Am7 ");

        assert_eq!(
            screen.handle_key_event(key(leave)),
            GuitarArticulationAction::Play(Take::Converted),
            "{leave:?}"
        );
        assert!(screen.arp_materials_input().is_none());
        assert!(screen.arp_overlay_open());
        assert_eq!(screen.arp_materials(), ["C", "Am7"]);
        assert_eq!(screen.arp_material(), "Am7");
        assert_eq!(screen.mml(), "l16cdef");
        assert!(screen.take_unsaved_settings());
        assert!(!screen.take_unsaved_settings(), "1 回だけ");
    }
}

#[test]
fn empty_lines_are_dropped_and_an_empty_cursor_line_plays_nothing() {
    let mut screen = overlay_screen("l16cdef", &["C", "G"]);
    screen.handle_key_event(key(KeyCode::Tab));
    // 最後の行末にいるので、空行を 2 つ足す。
    screen.handle_key_event(key(KeyCode::Enter));
    screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Tab)),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.arp_materials(), ["C", "G"]);
    assert!(!screen.take_unsaved_settings());
    assert_eq!(screen.mml(), "l16cdef");
}

#[test]
fn the_current_material_on_the_cursor_line_is_not_played_again() {
    let mut screen = overlay_screen("G", &["C", "G"]);
    screen.handle_key_event(key(KeyCode::Tab));
    let history = screen.history().clone();
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Tab)),
        GuitarArticulationAction::Continue
    );
    assert_eq!(*screen.history(), history);
}

#[test]
fn a_material_that_cannot_be_read_keeps_the_list_and_shows_why() {
    let mut screen = overlay_screen("l16cdef", &[]);
    screen.handle_key_event(key(KeyCode::Tab));
    type_text(&mut screen, "@@@");

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Tab)),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.arp_materials(), ["@@@"]);
    assert_eq!(screen.mml(), "l16cdef");
    assert!(screen.error.is_some());
}
