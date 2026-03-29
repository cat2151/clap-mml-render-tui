use super::*;

#[test]
fn handle_normal_shift_h_enters_notepad_history_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["l8cdef".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["o5g".to_string()];

    let result =
        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::NotepadHistory));
    assert!(matches!(app.notepad_focus, PatchPhrasePane::History));
    assert_eq!(app.notepad_history_state.selected(), Some(0));
    assert_eq!(app.notepad_favorites_state.selected(), Some(0));
}

#[test]
fn handle_normal_h_no_longer_enters_notepad_history_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["l8cdef".to_string()];

    let result = app.handle_normal(KeyCode::Char('h'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_enter_records_notepad_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["l8cdef".to_string()];

    app.handle_normal(KeyCode::Enter);

    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["l8cdef".to_string()]
    );
}

#[test]
fn handle_patch_select_enter_records_notepad_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];

    app.handle_patch_select(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
}

#[test]
fn handle_patch_phrase_enter_records_notepad_history() {
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

    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
}

#[test]
fn handle_notepad_history_j_previews_without_reordering_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('j'));

    assert_eq!(app.notepad_history_cursor, 1);
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["alpha".to_string(), "beta".to_string()]
    );
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "beta"
    ));
}

#[test]
fn handle_notepad_history_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "zero".to_string(),
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
        "four".to_string(),
        "five".to_string(),
    ];
    app.notepad_history_page_size = 2;
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('j'));

    app.handle_notepad_history(KeyCode::PageDown);
    assert_eq!(app.notepad_history_cursor, 3);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "three"
    ));

    app.handle_notepad_history(KeyCode::PageUp);
    assert_eq!(app.notepad_history_cursor, 1);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "one"
    ));
}

#[test]
fn handle_notepad_history_page_up_at_top_does_not_repreview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.notepad_history_page_size = 2;
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::PageUp);

    assert_eq!(app.notepad_history_cursor, 0);
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));
}

#[test]
fn handle_notepad_history_enter_overwrites_current_line_and_closes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad.history = vec!["after".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(app.lines, vec!["after".to_string()]);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "after"
    ));
}

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

#[test]
fn handle_notepad_history_f_adds_selected_history_to_favorites() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('j'));

    app.handle_notepad_history(KeyCode::Char('f'));

    assert_eq!(
        app.patch_phrase_store.notepad.favorites,
        vec!["beta".to_string()]
    );
}

#[test]
fn handle_notepad_history_right_switches_focus_to_favorites() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["beta".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Right);

    assert!(matches!(app.notepad_focus, PatchPhrasePane::Favorites));
    assert_eq!(app.notepad_history_state.selected(), Some(0));
    assert_eq!(app.notepad_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "beta"
    ));
}

#[test]
fn handle_notepad_history_left_switches_focus_to_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["beta".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Right);

    app.handle_notepad_history(KeyCode::Left);

    assert!(matches!(app.notepad_focus, PatchPhrasePane::History));
    assert_eq!(app.notepad_history_state.selected(), Some(0));
    assert_eq!(app.notepad_favorites_state.selected(), Some(0));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "alpha"
    ));
}

#[test]
fn handle_notepad_history_dd_removes_favorite_and_moves_it_to_history_top() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.patch_phrase_store.notepad.favorites = vec!["beta".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('l'));

    app.handle_notepad_history(KeyCode::Char('d'));
    assert!(app.notepad_pending_delete);
    app.handle_notepad_history(KeyCode::Char('d'));

    assert!(!app.notepad_pending_delete);
    assert!(app.patch_phrase_store.notepad.favorites.is_empty());
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec!["beta".to_string(), "alpha".to_string()]
    );
}

#[test]
fn handle_notepad_history_d_does_not_arm_delete_when_favorites_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('l'));

    app.handle_notepad_history(KeyCode::Char('d'));

    assert!(!app.notepad_pending_delete);
    assert_eq!(app.notepad_favorites_state.selected(), None);
}

#[test]
fn handle_notepad_history_question_mark_enters_help_and_esc_returns_to_history() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::NotepadHistory));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::NotepadHistory));
}
