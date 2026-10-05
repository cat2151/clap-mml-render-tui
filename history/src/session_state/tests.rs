use super::*;

#[test]
fn missing_session_uses_defaults_without_writing() {
    let _dirs = crate::test_support::temp_local_dirs("session_first_start");
    assert_eq!(
        load_session_state().unwrap().active_screen,
        crate::PrimaryScreen::Notepad
    );
    assert!(!crate::paths::session_state_path().unwrap().exists());
}

#[test]
fn invalid_session_and_unknown_screen_cannot_be_overwritten_by_guide_dates() {
    let _dirs = crate::test_support::temp_local_dirs("session_invalid");
    let path = crate::paths::session_state_path().unwrap();
    for json in ["{", r#"{"active_screen":"future_screen"}"#] {
        std::fs::write(&path, json).unwrap();
        let error = load_session_state().unwrap_err();
        assert!(format!("{error:#}").contains(&path.display().to_string()));
        assert!(save_keyboard_note_guide_overlay_date("2026-10-05").is_err());
        assert!(save_notepad_sound_check_guide_overlay_date("2026-10-05").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
    }
}
