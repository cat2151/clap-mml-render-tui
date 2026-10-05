use super::apply_startup_keyboard;
use crate::history::KeyboardSessionState;
use crate::screen_switch::PrimaryScreen;
use cmrt_tui_core::keyboard_session_state::{
    KeyboardControllerState, ModulationMode, NotePlaybackMode,
};

#[test]
fn unreadable_notepad_stops_startup_without_replacing_existing_data() {
    let (_tmp, _guard) = cmrt_history::test_support::temp_local_dirs("startup_invalid_notepad");
    let path = crate::test_utils::session_state_path_for_test()
        .unwrap()
        .with_file_name("notepad.json");
    std::fs::write(&path, r#"{"lines":["saved phrase"]"#).unwrap();
    let before = std::fs::read(&path).unwrap();
    let cfg = crate::tui::tests::test_config();
    let error = crate::tui::TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none())
        .err()
        .expect("invalid document must stop startup");
    assert!(format!("{error:#}").contains("notepad.json"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!crate::test_utils::session_state_path_for_test()
        .unwrap()
        .exists());
}

#[test]
fn unknown_saved_screen_stops_startup_without_losing_the_separate_document() {
    let (_tmp, _guard) = cmrt_history::test_support::temp_local_dirs("startup_unknown_screen");
    crate::history::save_notepad_document(&crate::history::NotepadDocument {
        cursor: 1,
        lines: vec!["first saved phrase".into(), "second saved phrase".into()],
    })
    .unwrap();
    let session_path = crate::test_utils::session_state_path_for_test().unwrap();
    let document_path = session_path.with_file_name("notepad.json");
    let before = std::fs::read(&document_path).unwrap();
    let invalid = r#"{"active_screen":"future_screen"}"#;
    std::fs::write(&session_path, invalid).unwrap();
    let cfg = crate::tui::tests::test_config();
    assert!(crate::tui::TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none()).is_err());
    assert_eq!(std::fs::read(&document_path).unwrap(), before);
    assert_eq!(std::fs::read_to_string(&session_path).unwrap(), invalid);

    crate::history::save_session_state(&crate::history::SessionState::default()).unwrap();
    let loaded = super::load_initial_session_state().unwrap();
    assert_eq!(loaded.cursor, 1);
    assert_eq!(
        loaded.lines,
        vec!["first saved phrase", "second saved phrase"]
    );
}

fn saved_keyboard() -> KeyboardSessionState {
    KeyboardSessionState {
        patch: Some("saved.fxp".to_string()),
        buffer_multiplier: 8,
        note_playback_mode: NotePlaybackMode::Repeat,
        repeat_chords: vec![vec![62]],
        mml: "d".to_string(),
        effect_chain: vec![serde_json::json!({"TONE3000 preset": "x"})],
        patch_filter: "p:surge".to_string(),
        controllers: KeyboardControllerState {
            modulation: ModulationMode::Periodic,
            ..KeyboardControllerState::default()
        },
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
