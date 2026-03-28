use super::*;

#[test]
fn extract_patch_phrase_reads_patch_name_and_phrase() {
    let result =
        TuiApp::extract_patch_phrase(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}  l8cdef"#).unwrap();

    assert_eq!(result.0, "Pads/Pad 1.fxp");
    assert_eq!(result.1, "l8cdef");
}

#[test]
fn handle_patch_phrase_enter_replays_current_preview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Enter);

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_phrase_space_replays_current_preview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char(' '));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_phrase_i_from_history_enters_insert_with_preview_mml() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('i'));

    assert!(matches!(app.mode, Mode::Insert));
    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
    assert_eq!(
        app.textarea.lines().join(""),
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    );
}

#[test]
fn handle_patch_phrase_i_from_favorites_stays_in_patch_phrase() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('l'));

    app.handle_patch_phrase(KeyCode::Char('i'));

    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.lines, vec!["before".to_string()]);
}

#[test]
fn record_patch_phrase_history_uses_phrase_without_embedded_json() {
    let mut app = TuiApp::new_for_test(test_config());

    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8efga"#);
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let stored = app
        .patch_phrase_store
        .patches
        .get("Pads/Pad 1.fxp")
        .expect("patch history should be stored");
    assert_eq!(
        stored.history,
        vec!["l8cdef".to_string(), "l8efga".to_string()]
    );
    assert!(stored.favorites.is_empty());
}

#[test]
fn record_patch_phrase_history_truncates_to_recent_100_items() {
    let mut app = TuiApp::new_for_test(test_config());

    for i in 0..105 {
        app.record_patch_phrase_history(&format!(
            r#"{{"Surge XT patch":"Pads/Pad 1.fxp"}} l8c{}"#,
            i
        ));
    }

    let stored = app
        .patch_phrase_store
        .patches
        .get("Pads/Pad 1.fxp")
        .expect("patch history should be stored");
    assert!(app.patch_phrase_store_dirty);
    assert_eq!(stored.history.len(), 100);
    assert_eq!(stored.history.first().map(String::as_str), Some("l8c104"));
    assert_eq!(stored.history.last().map(String::as_str), Some("l8c5"));
}

#[test]
fn patch_phrase_store_flushes_only_when_requested() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_patch_phrase_flush_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_data_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let patch_history_path = dirs::data_local_dir()
        .expect("data local dir should resolve in isolated test")
        .join("clap-mml-render-tui")
        .join("patch_history.json");
    assert!(
        !patch_history_path.exists(),
        "patch history should not be written until flush is requested"
    );
    assert!(app.patch_phrase_store_dirty);

    app.flush_patch_phrase_store_if_dirty();

    assert!(patch_history_path.exists());
    assert!(!app.patch_phrase_store_dirty);
    let loaded = crate::history::load_patch_phrase_store();
    let stored = loaded
        .patches
        .get("Pads/Pad 1.fxp")
        .expect("flushed patch history should be persisted");
    assert_eq!(stored.history, vec!["l8cdef".to_string()]);

    std::fs::remove_dir_all(&tmp).ok();
}
