use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::test_effects::{amp_plugins, amp_stage};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 既定の MML を確定し、MML 欄を閉じた画面。
fn screen_with_amps() -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::with_effect_plugins(amp_plugins());
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));
    screen
}

#[test]
fn x_without_an_effect_catalog_explains_instead_of_opening() {
    let mut screen = GuitarArticulationScreen::default();
    screen.enter();
    screen.handle_key_event(key(KeyCode::Esc));

    screen.handle_key_event(key(KeyCode::Char('x')));

    assert!(!screen.effect_overlay_open());
    assert_eq!(
        screen.error.as_deref(),
        Some(cmrt_effect_chain_select::messages::NOT_AVAILABLE_ON_THIS_BACKEND)
    );
}

#[test]
fn adding_an_amp_previews_the_articulated_take_through_it() {
    let mut screen = screen_with_amps();
    screen.handle_key_event(key(KeyCode::Char('x')));

    let action = screen.handle_key_event(key(KeyCode::Char('a')));

    assert_eq!(
        action,
        GuitarArticulationAction::PreviewEffectChain(vec![amp_stage()])
    );
    // 確定するまで画面の chain は変わらない。
    assert!(screen.effect_chain().is_empty());
}

#[test]
fn enter_twice_commits_the_chain_and_plays_the_articulated_take() {
    let mut screen = screen_with_amps();
    screen.handle_key_event(key(KeyCode::Char('x')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Enter));

    let action = screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert!(!screen.effect_overlay_open());
    assert_eq!(screen.effect_chain(), [amp_stage()]);
}

#[test]
fn esc_discards_the_edited_chain() {
    let mut screen = screen_with_amps();
    screen.handle_key_event(key(KeyCode::Char('x')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Enter));

    screen.handle_key_event(key(KeyCode::Esc));

    assert!(!screen.effect_overlay_open());
    assert!(screen.effect_chain().is_empty());
}

#[test]
fn keys_go_to_the_overlay_while_it_is_open() {
    let mut screen = screen_with_amps();
    screen.handle_key_event(key(KeyCode::Char('x')));

    // 画面の `q`（終了）ではなく、overlay は何もしない。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        GuitarArticulationAction::Continue
    );
    screen.handle_key_event(key(KeyCode::Char('?')));
    assert!(screen.help_open());
    assert!(screen.effect_overlay_open());
}

#[test]
fn the_list_filter_uses_the_textarea_cursor() {
    let mut screen = screen_with_amps();
    screen.handle_key_event(key(KeyCode::Char('x')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    assert!(!screen.uses_textarea_cursor());

    screen.handle_key_event(key(KeyCode::Char('/')));

    assert!(screen.uses_textarea_cursor());
}

#[test]
fn previewing_without_mml_explains_instead_of_playing() {
    let mut screen = GuitarArticulationScreen::with_effect_plugins(amp_plugins());
    screen.handle_key_event(key(KeyCode::Char('x')));

    let action = screen.handle_key_event(key(KeyCode::Char('a')));

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert!(screen.error.is_some());
}
