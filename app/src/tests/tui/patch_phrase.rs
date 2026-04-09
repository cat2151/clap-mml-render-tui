use super::*;

#[test]
fn extract_patch_phrase_reads_patch_name_and_phrase() {
    let result =
        TuiApp::extract_patch_phrase(r#"{"Surge XT patch":"Pads/Pad 1.fxp"}  l8cdef"#).unwrap();

    assert_eq!(result.0, "Pads/Pad 1.fxp");
    assert_eq!(result.1, "l8cdef");
}

#[test]
fn handle_patch_phrase_enter_inserts_preview_above_current_line_and_closes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "top".to_string(),
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(
        app.lines,
        vec![
            "top".to_string(),
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string(),
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()
        ]
    );
    assert_eq!(app.cursor, 1);
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
fn handle_patch_phrase_arrow_keys_switch_focus_and_preview() {
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

    app.handle_patch_phrase(KeyCode::Right);

    assert!(matches!(app.patch_phrase_focus, PatchPhrasePane::Favorites));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} o5g"#
    ));

    app.handle_patch_phrase(KeyCode::Left);

    assert!(matches!(app.patch_phrase_focus, PatchPhrasePane::History));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_phrase_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "zero".to_string(),
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
                "four".to_string(),
            ],
            favorites: vec!["fav".to_string()],
        },
    );
    app.patch_phrase_page_size = 2;
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('j'));

    app.handle_patch_phrase(KeyCode::PageDown);
    assert_eq!(app.patch_phrase_history_cursor, 3);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} three"#
    ));

    app.handle_patch_phrase(KeyCode::PageUp);
    assert_eq!(app.patch_phrase_history_cursor, 1);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} one"#
    ));
}

#[test]
fn handle_patch_phrase_slash_then_enter_keeps_filtered_results_for_j_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "alpha".to_string(),
                "beta jk".to_string(),
                "gamma jk".to_string(),
            ],
            favorites: vec!["fav".to_string()],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('/'));
    app.handle_patch_phrase(KeyCode::Char('j'));
    app.handle_patch_phrase(KeyCode::Char('k'));
    app.handle_patch_phrase(KeyCode::Enter);
    app.handle_patch_phrase(KeyCode::Char('j'));

    assert!(!app.patch_phrase_filter_active);
    assert_eq!(app.patch_phrase_query, "jk");
    assert_eq!(
        app.patch_phrase_history_items(),
        vec!["beta jk".to_string(), "gamma jk".to_string()]
    );
    assert_eq!(app.patch_phrase_history_cursor, 1);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} gamma jk"#
    ));
}

#[test]
fn handle_patch_phrase_allows_slash_character_in_filter_query() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "alpha".to_string(),
                "dir/name".to_string(),
                "dir other".to_string(),
            ],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('/'));
    app.handle_patch_phrase(KeyCode::Char('/'));
    app.handle_patch_phrase(KeyCode::Char('n'));

    assert!(app.patch_phrase_filter_active);
    assert_eq!(app.patch_phrase_query, "/n");
    assert_eq!(
        app.patch_phrase_history_items(),
        vec!["dir/name".to_string()]
    );
}

#[test]
fn handle_patch_phrase_left_in_filter_query_does_not_repreview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["alpha".to_string(), "beta".to_string()],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('/'));
    app.handle_patch_phrase(KeyCode::Char('b'));
    let play_state_before = app.play_state.lock().unwrap().clone();

    app.handle_patch_phrase(KeyCode::Left);

    assert!(app.patch_phrase_filter_active);
    assert_eq!(app.patch_phrase_query, "b");
    assert!(*app.play_state.lock().unwrap() == play_state_before);
}

#[test]
fn handle_patch_phrase_n_p_t_switch_to_corresponding_overlays() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
        "Leads/Lead 1.fxp",
    ]))));
    app.patch_phrase_store.notepad.history = vec!["from history".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::NotepadHistory));

    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('p'));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Pads/Pad 1.fxp"));

    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('t'));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_filtered[app.patch_cursor], "Pads/Pad 1.fxp");
}

#[test]
fn handle_patch_phrase_page_up_at_top_does_not_repreview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["zero".to_string(), "one".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );
    app.patch_phrase_page_size = 2;
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::PageUp);

    assert_eq!(app.patch_phrase_history_cursor, 0);
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));
    assert!(app.patch_phrase_store.notepad.history.is_empty());
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
fn record_patch_phrase_history_resolves_factory_prefixed_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));

    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let stored = app
        .patch_phrase_store
        .patches
        .get("patches_factory/Pads/Pad 1.fxp")
        .expect("patch history should be stored with prefixed patch name");
    assert_eq!(stored.history, vec!["l8cdef".to_string()]);
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
}

#[test]
fn start_patch_phrase_migrates_existing_history_and_favorites_to_prefixed_patch_name() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Pads/Pad 1.fxp",
    ]))));
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["hist".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );

    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    assert_eq!(
        app.patch_phrase_name.as_deref(),
        Some("patches_factory/Pads/Pad 1.fxp")
    );
    let stored = app
        .patch_phrase_store
        .patches
        .get("patches_factory/Pads/Pad 1.fxp")
        .expect("migrated patch state should exist");
    assert_eq!(stored.history, vec!["hist".to_string()]);
    assert_eq!(stored.favorites, vec!["fav".to_string()]);
    assert!(!app
        .patch_phrase_store
        .patches
        .contains_key("Pads/Pad 1.fxp"));
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
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.record_patch_phrase_history(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#);

    let patch_history_path = dirs::config_local_dir()
        .expect("config local dir should resolve in isolated test")
        .join("clap-mml-render-tui")
        .join("history")
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

#[test]
fn handle_patch_phrase_question_mark_enters_help_and_esc_returns_to_patch_phrase() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::PatchPhrase));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::PatchPhrase));
}
