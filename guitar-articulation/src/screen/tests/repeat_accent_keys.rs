use super::{key, screen_with_mml};
use crate::{AccentPattern, GuitarArticulationAction, RowRule, RuleTable, Take};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

#[test]
fn shift_r_starts_the_repeat_with_the_converted_take_and_stops_it_again() {
    let mut screen = screen_with_mml("l16cdef");
    assert!(!screen.repeat());

    assert_eq!(
        screen.handle_key_event(shift('R')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.repeat());

    assert_eq!(
        screen.handle_key_event(shift('R')),
        GuitarArticulationAction::StopRepeat
    );
    assert!(!screen.repeat());
}

#[test]
fn shift_r_repeats_only_the_cursor_column_in_the_note_preview() {
    let mut screen = screen_with_mml("l16cdef");
    screen.handle_key_event(key(KeyCode::Char('n')));
    assert_eq!(
        screen.handle_key_event(shift('R')),
        GuitarArticulationAction::PlayNote {
            take: Take::Converted,
            column: 0
        }
    );
}

#[test]
fn shift_a_cycles_the_accent_pattern_and_plays_the_new_take() {
    let mut screen = screen_with_mml("l16cedfc");
    screen.handle_key_event(key(KeyCode::Char('e')));
    assert!(screen.rules().is_row_on(RowRule::EconomyPicking));
    let accents = |screen: &crate::GuitarArticulationScreen| -> Vec<bool> {
        screen.articulated().iter().map(|a| a.accent).collect()
    };
    assert_eq!(accents(&screen), vec![true, true, false, true, false]);

    assert_eq!(
        screen.handle_key_event(shift('A')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Bottom);
    assert_eq!(accents(&screen), vec![true, false, true, false, false]);

    screen.handle_key_event(shift('A'));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Both);
    assert_eq!(accents(&screen), vec![true, true, true, true, false]);

    screen.handle_key_event(shift('A'));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Top);
}

#[test]
fn the_accent_pattern_survives_a_new_mml_and_the_json() {
    let mut screen = screen_with_mml("l16cedfc");
    screen.handle_key_event(shift('A'));
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Bottom);

    let json = screen.rules().to_json();
    assert!(json.contains(r#""accent":"bottom""#), "{json}");
    assert_eq!(
        RuleTable::from_json(&json).unwrap().accent_pattern(),
        AccentPattern::Bottom
    );
    // 既定は書かない（保存済みの JSON の綴りを変えない）。
    assert!(!RuleTable::default().to_json().contains("accent"));
}
