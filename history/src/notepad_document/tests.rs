use super::*;

#[test]
fn a_missing_document_is_a_first_start_without_creating_a_file() {
    let _dirs = crate::test_support::temp_local_dirs("notepad_first_start");
    let path = crate::paths::notepad_document_path().unwrap();
    assert_eq!(load_notepad_document().unwrap(), NotepadDocument::default());
    assert!(!path.exists());
}

#[test]
fn document_round_trip_preserves_lines_and_cursor_without_settings() {
    let _dirs = crate::test_support::temp_local_dirs("notepad_round_trip");
    let document = NotepadDocument {
        cursor: 1,
        lines: vec!["cde".into(), "fga".into()],
    };
    save_notepad_document(&document).unwrap();
    assert_eq!(load_notepad_document().unwrap(), document);
    assert!(!crate::paths::session_state_path().unwrap().exists());
}

#[test]
fn invalid_existing_documents_are_reported_and_left_untouched() {
    let _dirs = crate::test_support::temp_local_dirs("notepad_invalid");
    let path = crate::paths::notepad_document_path().unwrap();
    for json in ["{", "{}", r#"{"lines":null}"#, r#"{"lines":[]}"#] {
        std::fs::write(&path, json).unwrap();
        let error = load_notepad_document().unwrap_err();
        assert!(format!("{error:#}").contains(&path.display().to_string()));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
    }
}

#[test]
fn an_unreadable_document_is_not_a_first_start() {
    let _dirs = crate::test_support::temp_local_dirs("notepad_unreadable");
    let path = crate::paths::notepad_document_path().unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(load_notepad_document().is_err());
    assert!(path.is_dir());
}

#[test]
fn guide_date_updates_and_session_writes_leave_the_document_untouched() {
    let _dirs = crate::test_support::temp_local_dirs("notepad_separate_settings");
    save_notepad_document(&NotepadDocument {
        cursor: 1,
        lines: vec!["edited phrase".into(), "another phrase".into()],
    })
    .unwrap();
    let path = crate::paths::notepad_document_path().unwrap();
    let before = std::fs::read(&path).unwrap();
    crate::save_keyboard_note_guide_overlay_date("2026-10-05").unwrap();
    crate::save_notepad_sound_check_guide_overlay_date("2026-10-05").unwrap();
    crate::save_session_state(&crate::SessionState {
        active_screen: crate::PrimaryScreen::DrumSequencer,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
