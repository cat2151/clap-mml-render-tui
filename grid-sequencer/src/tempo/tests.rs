use std::time::Instant;

use cmrt_tui_core::bpm::BpmMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{GridSequencerScreen, STEP_INTERVAL};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn ctrl_b_opens_input_and_enter_applies_manual_bpm() {
    let mut screen = GridSequencerScreen::new(None);
    let now = Instant::now();
    screen.handle_key(
        key(KeyCode::Char('b'), KeyModifiers::CONTROL),
        now,
        &crate::tests::ready_ctx(&[]),
    );
    for character in "128.125".chars() {
        screen.handle_key(
            key(KeyCode::Char(character), KeyModifiers::NONE),
            now,
            &crate::tests::ready_ctx(&[]),
        );
    }
    screen.handle_key(
        key(KeyCode::Enter, KeyModifiers::NONE),
        now,
        &crate::tests::ready_ctx(&[]),
    );

    assert_eq!(screen.bpm_mode(), BpmMode::Manual(128.125));
    assert_eq!(screen.bpm(), 128.125);
    assert!(screen.state.is_running());
}

#[test]
fn a_returns_to_auto_and_plain_b_keeps_its_existing_binding() {
    let mut screen = GridSequencerScreen::new(None);
    screen.bpm_mode = BpmMode::Manual(90.0);
    let now = Instant::now();
    let ctx = crate::tests::ready_ctx(&[]);
    screen.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE), now, &ctx);
    assert!(screen.single_buffering());
    assert!(screen.bpm_input.is_none());

    screen.handle_key(key(KeyCode::Char('b'), KeyModifiers::CONTROL), now, &ctx);
    screen.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE), now, &ctx);
    assert_eq!(screen.bpm_mode(), BpmMode::Auto);
    assert_eq!(screen.bpm(), crate::BPM);
}

#[test]
fn dynamic_step_interval_matches_the_selected_bpm() {
    assert_eq!(
        crate::step_interval_at(120.0),
        std::time::Duration::from_millis(125)
    );
    assert_ne!(crate::step_interval_at(120.0), STEP_INTERVAL);
}
