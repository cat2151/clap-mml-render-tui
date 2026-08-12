use cmrt_loop_browser_domain::time_stretch::TARGET_BPM;
use cmrt_tui_core::bpm::{BpmMode, BpmRange};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{LoopBrowser, LoopBrowserAction};

fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

/// Ctrl+B を開いて `input` を打ち、Enter で確定する。
fn enter_bpm(browser: &mut LoopBrowser, input: &str) -> LoopBrowserAction {
    browser.handle_key_event(press(KeyCode::Char('b'), KeyModifiers::CONTROL));
    for character in input.chars() {
        browser.handle_key_event(press(KeyCode::Char(character), KeyModifiers::NONE));
    }
    browser.handle_key_event(press(KeyCode::Enter, KeyModifiers::NONE))
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
            mode: BpmMode::Auto(_),
            ..
        }
    ));
    // 範囲を指定していないので、従来どおり120へ寄せる自動モードへ戻る。
    assert_eq!(browser.bpm_mode(), BpmMode::Auto(TARGET_BPM));
}

#[test]
fn escape_cancels_without_changing_the_mode() {
    let mut browser = LoopBrowser::default();
    browser.handle_key_event(press(KeyCode::Char('b'), KeyModifiers::CONTROL));
    assert!(matches!(
        browser.handle_key_event(press(KeyCode::Esc, KeyModifiers::NONE)),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.bpm_mode(), BpmMode::Auto(TARGET_BPM));
    assert!(browser.bpm_input.is_none());
}

#[test]
fn a_hyphen_pair_sets_the_range_and_draws_inside_it() {
    let mut browser = LoopBrowser::default();
    assert_eq!(browser.bpm_range(), BpmRange::fixed(TARGET_BPM));

    let action = enter_bpm(&mut browser, "90-140");
    assert!(matches!(
        action,
        LoopBrowserAction::BpmChanged {
            mode: BpmMode::Auto(bpm),
            ..
        } if (90.0..=140.0).contains(&bpm)
    ));
    assert_eq!(browser.bpm_range(), BpmRange::new(90.0, 140.0).unwrap());

    // 範囲は A キーでの引き直しをまたいで残る。
    let mut drawn = std::collections::HashSet::new();
    for _ in 0..48 {
        browser.handle_key_event(press(KeyCode::Char('b'), KeyModifiers::CONTROL));
        browser.handle_key_event(press(KeyCode::Char('a'), KeyModifiers::NONE));
        let bpm = browser.bpm_mode().bpm();
        assert!((90.0..=140.0).contains(&bpm), "bpm={bpm}");
        drawn.insert(bpm as i64);
    }
    assert_eq!(browser.bpm_range(), BpmRange::new(90.0, 140.0).unwrap());
    assert!(drawn.len() > 1, "A で引き直されていない: {drawn:?}");
}

#[test]
fn a_manual_bpm_replaces_the_automatic_draw_but_keeps_the_range() {
    let mut browser = LoopBrowser::default();
    enter_bpm(&mut browser, "90-140");
    enter_bpm(&mut browser, "128");
    assert_eq!(browser.bpm_mode(), BpmMode::Manual(128.0));
    assert_eq!(browser.bpm_range(), BpmRange::new(90.0, 140.0).unwrap());
}
