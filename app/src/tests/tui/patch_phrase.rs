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
    app.editor.lines = vec![
        "top".to_string(),
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string(),
    ];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));
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
        app.editor.lines,
        vec![
            "top".to_string(),
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string(),
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()
        ]
    );
    assert_eq!(app.editor.cursor, 1);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_phrase_space_replays_current_preview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
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
    app.editor.lines = vec!["before".to_string()];
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
        app.editor.lines,
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()]
    );
    assert_eq!(
        app.editor.textarea.lines().join(""),
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    );
}

#[test]
fn handle_patch_phrase_i_from_favorites_stays_in_patch_phrase() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
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
    assert_eq!(app.editor.lines, vec!["before".to_string()]);
}

#[test]
fn handle_patch_phrase_arrow_keys_switch_focus_and_preview() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Right);

    assert!(matches!(app.patch_phrase.focus, PatchPhrasePane::Favorites));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} o5g"#
    ));

    app.handle_patch_phrase(KeyCode::Left);

    assert!(matches!(app.patch_phrase.focus, PatchPhrasePane::History));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#
    ));
}

#[test]
fn handle_patch_phrase_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
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
    app.patch_phrase.page_size = 2;
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.handle_patch_phrase(KeyCode::Char('j'));

    app.handle_patch_phrase(KeyCode::PageDown);
    assert_eq!(app.patch_phrase.history_cursor, 3);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} three"#
    ));

    app.handle_patch_phrase(KeyCode::PageUp);
    assert_eq!(app.patch_phrase.history_cursor, 1);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch":"Pads/Pad 1.fxp"} one"#
    ));
}

#[test]
fn handle_patch_phrase_j_prefetches_predicted_navigation_cache() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "zero".to_string(),
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
            ],
            favorites: vec![],
        },
    );
    app.patch_phrase.page_size = 2;
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('j'));

    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} zero"#));
    assert!(cache.contains_key(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} two"#));
    assert!(cache.contains_key(r#"{"Surge XT patch":"Pads/Pad 1.fxp"} three"#));
}

#[test]
fn handle_patch_phrase_j_prefetches_direction_first_then_fills_remaining_navigation_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: (0..12).map(|i| format!("phrase {i}")).collect(),
            favorites: vec![],
        },
    );
    app.patch_phrase.page_size = 5;
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());
    app.patch_phrase.history_cursor = 4;
    app.patch_phrase.history_state.select(Some(4));

    app.handle_patch_phrase(KeyCode::Char('j'));

    assert_eq!(
        app.audio_cache_order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 6"#,
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 7"#,
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 4"#,
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 10"#,
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 0"#,
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 8"#,
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} phrase 9"#,
        ]
    );
}

#[test]
fn handle_patch_phrase_starts_scrolling_before_cursor_reaches_view_edge() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "zero".to_string(),
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
                "four".to_string(),
                "five".to_string(),
                "six".to_string(),
                "seven".to_string(),
            ],
            favorites: vec![],
        },
    );
    app.patch_phrase.page_size = 6;
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    for _ in 0..4 {
        app.handle_patch_phrase(KeyCode::Char('j'));
    }
    assert_eq!(app.patch_phrase.history_cursor, 4);
    assert_eq!(app.patch_phrase.history_state.offset(), 1);

    for _ in 0..2 {
        app.handle_patch_phrase(KeyCode::Char('k'));
    }
    assert_eq!(app.patch_phrase.history_cursor, 2);
    assert_eq!(app.patch_phrase.history_state.offset(), 0);
}

#[path = "patch_phrase/filter_and_switching.rs"]
mod filter_and_switching;
