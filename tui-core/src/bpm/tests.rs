use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn enter(input: &str) -> (BpmInput, BpmInputAction) {
    let mut state = BpmInput::default();
    for character in input.chars() {
        assert_eq!(
            state.handle_key(press(KeyCode::Char(character))),
            BpmInputAction::Continue
        );
    }
    let action = state.handle_key(press(KeyCode::Enter));
    (state, action)
}

#[test]
fn accepts_integer_and_unbounded_decimal_precision() {
    assert_eq!(
        enter("128").1,
        BpmInputAction::Apply(BpmMode::Manual(128.0))
    );
    assert_eq!(
        enter("128.123456789").1,
        BpmInputAction::Apply(BpmMode::Manual(128.123456789))
    );
}

#[test]
fn accepts_the_inclusive_limits() {
    assert_eq!(enter("20").1, BpmInputAction::Apply(BpmMode::Manual(20.0)));
    assert_eq!(
        enter("300").1,
        BpmInputAction::Apply(BpmMode::Manual(300.0))
    );
}

#[test]
fn rejects_empty_invalid_and_out_of_range_values_without_closing() {
    for value in ["", ".", "19.99", "300.01"] {
        let (state, action) = enter(value);
        assert_eq!(action, BpmInputAction::Continue, "value={value}");
        assert!(state.error().is_some(), "value={value}");
    }
}

#[test]
fn ignores_a_second_decimal_point_and_supports_backspace() {
    let mut state = BpmInput::default();
    for character in "12.3.4".chars() {
        state.handle_key(press(KeyCode::Char(character)));
    }
    assert_eq!(state.buffer(), "12.34");
    state.handle_key(press(KeyCode::Backspace));
    assert_eq!(state.buffer(), "12.3");
}

#[test]
fn a_selects_auto_and_escape_cancels() {
    let mut state = BpmInput::default();
    assert_eq!(
        state.handle_key(press(KeyCode::Char('a'))),
        BpmInputAction::Apply(BpmMode::Auto)
    );
    assert_eq!(
        state.handle_key(press(KeyCode::Esc)),
        BpmInputAction::Cancel
    );
}

#[test]
fn saved_manual_values_are_validated() {
    assert_eq!(
        BpmMode::from_saved_manual(Some(128.5)),
        BpmMode::Manual(128.5)
    );
    assert_eq!(BpmMode::from_saved_manual(Some(0.0)), BpmMode::Auto);
    assert_eq!(BpmMode::from_saved_manual(Some(f64::NAN)), BpmMode::Auto);
}
