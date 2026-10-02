use super::apply_startup_keyboard;
use crate::history::KeyboardSessionState;
use crate::screen_switch::PrimaryScreen;
use cmrt_tui_core::keyboard_session_state::NotePlaybackMode;

fn saved_keyboard() -> KeyboardSessionState {
    KeyboardSessionState {
        patch: Some("saved.fxp".to_string()),
        buffer_multiplier: 8,
        note_playback_mode: NotePlaybackMode::Repeat,
        repeat_chords: vec![vec![62]],
        mml: "d".to_string(),
        effect_chain: vec![serde_json::json!({"TONE3000 preset": "x"})],
        patch_filter: "p:surge".to_string(),
    }
}

#[test]
fn startup_keyboard_replaces_keyboard_and_screen_but_keeps_buffer_multiplier() {
    let mut active_screen = PrimaryScreen::Notepad;
    let mut keyboard = saved_keyboard();
    let startup = KeyboardSessionState {
        patch: Some("startup.fxp".to_string()),
        note_playback_mode: NotePlaybackMode::Auto,
        repeat_chords: vec![vec![60, 65, 67]],
        ..KeyboardSessionState::default()
    };

    apply_startup_keyboard(&mut active_screen, &mut keyboard, Some(startup.clone()));

    assert_eq!(active_screen, PrimaryScreen::Keyboard);
    assert_eq!(
        keyboard,
        KeyboardSessionState {
            buffer_multiplier: 8,
            ..startup
        }
    );
}

#[test]
fn no_startup_keyboard_leaves_saved_state_as_is() {
    let mut active_screen = PrimaryScreen::Notepad;
    let mut keyboard = saved_keyboard();

    apply_startup_keyboard(&mut active_screen, &mut keyboard, None);

    assert_eq!(active_screen, PrimaryScreen::Notepad);
    assert_eq!(keyboard, saved_keyboard());
}
