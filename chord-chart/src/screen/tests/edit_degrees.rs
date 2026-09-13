//! `i` が host 側の進行 editor を開くための action だけを返すことを確かめる。

use super::*;

#[test]
fn i_returns_the_selected_sections_stable_id_without_mutating_the_song() {
    let mut screen = three_section_screen();
    screen.section_cursor = 1;
    let selected_id = screen.song.sections[1].id;
    let before = screen.song.clone();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('i'))),
        ChordChartAction::EditDegrees(selected_id)
    );

    assert_eq!(screen.song, before);
    assert!(!screen.line_input_open());
}

#[test]
fn i_does_nothing_without_a_section() {
    let mut screen = ChordChartScreen::new(Song::empty());

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('i'))),
        ChordChartAction::Continue
    );
    assert!(!screen.line_input_open());
}

#[test]
fn i_does_nothing_while_the_arrangement_pane_has_focus() {
    let mut screen = three_section_screen();
    screen.focus = Pane::Arrangement;

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('i'))),
        ChordChartAction::Continue
    );
    assert!(!screen.line_input_open());
}
