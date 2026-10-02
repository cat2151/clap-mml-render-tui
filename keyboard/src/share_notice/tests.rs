use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::clipboard::take_text_for_test;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::session_state::{KeyboardSessionState, NotePlaybackMode};
use crate::{
    share_command, KeyboardContext, KeyboardMmlInput, KeyboardNoteGuide, KeyboardScreen,
    KeyboardState, KeyboardVoicingLookup,
};

struct NoVoicing;

impl KeyboardVoicingLookup for NoVoicing {
    fn cached_voicing(&self, _patch: &str) -> Option<PatchVoicing> {
        None
    }
}

fn context(patch_load: &PatchLoadState) -> KeyboardContext<'_> {
    KeyboardContext {
        patch_dirs_configured: true,
        patch_load,
        voicing: &NoVoicing,
        catalog_notes: &[],
    }
}

fn session() -> KeyboardSessionState {
    KeyboardSessionState {
        patch: Some("Pads/Warm Pad.fxp".to_string()),
        buffer_multiplier: 4,
        note_playback_mode: NotePlaybackMode::Repeat,
        repeat_chords: cmrt_chord::note_progression("ceg").unwrap(),
        mml: "ceg".to_string(),
        effect_chain: vec![
            serde_json::json!({"Surge XT Effects preset": "Delay/Rhythmic 1.srgfx"}),
        ],
    }
}

fn screen(session: &KeyboardSessionState) -> KeyboardScreen<'static> {
    KeyboardScreen::new(
        None,
        KeyboardState::from_session(session.clone()),
        KeyboardMmlInput::restored(session.mml.clone()),
        KeyboardNoteGuide::new(None),
    )
    .with_effect_chain(session.effect_chain.clone())
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn release(code: KeyCode) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, KeyEventKind::Release)
}

#[test]
fn y_copies_the_share_command_of_the_saved_state_and_opens_the_notice() {
    let load = PatchLoadState::ready(Vec::new());
    let ctx = context(&load);
    let session = session();
    let mut screen = screen(&session);
    let _ = take_text_for_test();

    screen.handle_key(press(KeyCode::Char('y')), &ctx);

    let expected = share_command(&screen.session_state());
    assert_eq!(
        expected,
        "cmrt kb -p \"Pads/Warm Pad.fxp\" -t repeat -m ceg -e \"Surge XT Effects preset=Delay/Rhythmic 1.srgfx\""
    );
    assert_eq!(take_text_for_test().as_deref(), Some(expected.as_str()));
    assert_eq!(screen.share_notice(), Some(expected.as_str()));
}

#[test]
fn releasing_y_keeps_the_notice_open() {
    let load = PatchLoadState::ready(Vec::new());
    let ctx = context(&load);
    let mut screen = screen(&session());

    screen.handle_key(press(KeyCode::Char('y')), &ctx);
    screen.handle_key(release(KeyCode::Char('y')), &ctx);

    assert!(screen.share_notice().is_some());
}

#[test]
fn the_next_press_closes_the_notice_and_still_does_its_own_work() {
    let load = PatchLoadState::ready(Vec::new());
    let ctx = context(&load);
    let mut screen = screen(&session());

    screen.handle_key(press(KeyCode::Char('y')), &ctx);
    let velocity_mode = screen.state.velocity_mode();
    screen.handle_key(press(KeyCode::Char('v')), &ctx);
    assert_eq!(screen.share_notice(), None);
    assert_ne!(screen.state.velocity_mode(), velocity_mode);

    screen.handle_key(press(KeyCode::Char('y')), &ctx);
    screen.handle_key(press(KeyCode::Char('i')), &ctx);
    assert_eq!(screen.share_notice(), None);
    assert!(screen.mml_input.is_active());
}

#[test]
fn y_is_typed_as_text_while_an_input_field_is_open() {
    let load = PatchLoadState::ready(vec![(
        "Pads/Warm Pad.fxp".to_string(),
        "pads/warm pad.fxp".to_string(),
    )]);
    let ctx = context(&load);
    let mut screen = screen(&session());
    let _ = take_text_for_test();

    screen.handle_key(press(KeyCode::Char('i')), &ctx);
    let before = screen.mml_input.value();
    screen.handle_key(press(KeyCode::Char('y')), &ctx);
    assert_eq!(screen.mml_input.value(), format!("{before}y"));
    screen.handle_key(press(KeyCode::Esc), &ctx);

    screen.handle_key(press(KeyCode::Char('/')), &ctx);
    assert!(screen.patch_filter.is_active());
    screen.handle_key(press(KeyCode::Char('y')), &ctx);
    assert_eq!(screen.patch_filter.value(), "y");

    assert_eq!(screen.share_notice(), None);
    assert_eq!(take_text_for_test(), None);
}
