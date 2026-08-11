use cmrt_tui_core::bpm::BpmMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{LoopBrowser, LoopBrowserAction};

fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn ctrl_b_accepts_a_decimal_manual_bpm_from_either_pane() {
    let mut browser = LoopBrowser::default();
    assert!(matches!(
        browser.handle_key_event(press(KeyCode::Char('b'), KeyModifiers::CONTROL)),
        LoopBrowserAction::Continue
    ));
    for character in "127.123456".chars() {
        browser.handle_key_event(press(KeyCode::Char(character), KeyModifiers::NONE));
    }
    let action = browser.handle_key_event(press(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(
        action,
        LoopBrowserAction::BpmChanged {
            mode: BpmMode::Manual(value),
            ..
        } if value == 127.123456
    ));
    assert_eq!(browser.bpm_mode(), BpmMode::Manual(127.123456));
}

#[test]
fn a_returns_to_automatic_bpm() {
    let mut browser = LoopBrowser::default();
    browser.set_bpm_mode(BpmMode::Manual(90.0));
    browser.handle_key_event(press(KeyCode::Char('b'), KeyModifiers::CONTROL));
    let action = browser.handle_key_event(press(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(matches!(
        action,
        LoopBrowserAction::BpmChanged {
            mode: BpmMode::Auto,
            ..
        }
    ));
    assert_eq!(browser.bpm_mode(), BpmMode::Auto);
}

#[test]
fn escape_cancels_without_changing_the_mode() {
    let mut browser = LoopBrowser::default();
    browser.handle_key_event(press(KeyCode::Char('b'), KeyModifiers::CONTROL));
    assert!(matches!(
        browser.handle_key_event(press(KeyCode::Esc, KeyModifiers::NONE)),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.bpm_mode(), BpmMode::Auto);
    assert!(browser.bpm_input.is_none());
}
