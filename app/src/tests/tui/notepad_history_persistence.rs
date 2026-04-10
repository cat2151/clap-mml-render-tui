use super::*;

#[test]
fn handle_notepad_history_enter_flushes_store() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_notepad_history_enter_flush_{}_{}",
        std::process::id(),
        unique
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad.history = vec!["after".to_string()];
    app.patch_phrase_store_dirty = true;
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Enter);

    let loaded = crate::history::load_patch_phrase_store();
    assert_eq!(
        loaded.notepad.history.first().map(String::as_str),
        Some("after")
    );
    assert!(!app.patch_phrase_store_dirty);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn handle_notepad_history_esc_flushes_store() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_notepad_history_esc_flush_{}_{}",
        std::process::id(),
        unique
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["after".to_string()];
    app.patch_phrase_store_dirty = true;
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Esc);

    let loaded = crate::history::load_patch_phrase_store();
    assert_eq!(
        loaded.notepad.history.first().map(String::as_str),
        Some("after")
    );
    assert!(!app.patch_phrase_store_dirty);

    let _ = std::fs::remove_dir_all(&tmp);
}
