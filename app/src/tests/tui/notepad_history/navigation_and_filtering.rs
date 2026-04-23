use super::*;

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
fn handle_notepad_history_j_prefetches_predicted_navigation_cache() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "zero".to_string(),
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
    ];
    app.notepad_history_page_size = 2;
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('j'));

    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key("zero"));
    assert!(cache.contains_key("two"));
    assert!(cache.contains_key("three"));
}

#[test]
fn handle_notepad_history_j_prefetches_direction_first_then_fills_remaining_navigation_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = (0..12).map(|i| format!("history {i}")).collect();
    app.notepad_history_page_size = 5;
    app.start_notepad_history();
    app.notepad_history_cursor = 4;
    app.notepad_history_state.select(Some(4));

    app.handle_notepad_history(KeyCode::Char('j'));

    assert_eq!(
        app.audio_cache_order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "history 6",
            "history 7",
            "history 4",
            "history 10",
            "history 0",
            "history 8",
            "history 9",
        ]
    );
}

#[test]
fn handle_notepad_history_k_prefetches_page_up_before_page_down_then_far_direction_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = (0..12).map(|i| format!("history {i}")).collect();
    app.notepad_history_page_size = 5;
    app.start_notepad_history();
    app.notepad_history_cursor = 6;
    app.notepad_history_state.select(Some(6));

    app.handle_notepad_history(KeyCode::Char('k'));

    assert_eq!(
        app.audio_cache_order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![
            "history 4",
            "history 3",
            "history 6",
            "history 0",
            "history 10",
            "history 2",
            "history 1",
        ]
    );
}

#[test]
fn handle_notepad_history_space_previews_selected_item() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta".to_string()];
    app.start_notepad_history();

    assert_eq!(app.notepad_history_cursor, 0);
    assert!(matches!(&*app.play_state.lock().unwrap(), PlayState::Idle));

    app.handle_notepad_history(KeyCode::Char(' '));

    assert_eq!(app.notepad_history_cursor, 0);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "alpha"
    ));
}

#[test]
fn handle_notepad_history_slash_then_enter_keeps_filtered_results_for_j_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "alpha".to_string(),
        "beta jk".to_string(),
        "gamma jk".to_string(),
    ];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('j'));
    app.handle_notepad_history(KeyCode::Char('k'));
    app.handle_notepad_history(KeyCode::Enter);
    app.handle_notepad_history(KeyCode::Char('j'));

    assert!(!app.notepad_filter_active);
    assert_eq!(app.notepad_query, "jk");
    assert_eq!(
        app.notepad_history_items(),
        vec!["beta jk".to_string(), "gamma jk".to_string()]
    );
    assert_eq!(app.notepad_history_cursor, 1);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "gamma jk"
    ));
}

#[test]
fn handle_notepad_history_allows_slash_character_in_filter_query() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "alpha".to_string(),
        "dir/name".to_string(),
        "dir other".to_string(),
    ];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('n'));

    assert!(app.notepad_filter_active);
    assert_eq!(app.notepad_query, "/n");
    assert_eq!(app.notepad_history_items(), vec!["dir/name".to_string()]);
}

#[test]
fn handle_notepad_history_filter_space_updates_query_before_preview_shortcut() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string(), "beta soft".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('b'));
    app.handle_notepad_history(KeyCode::Char('e'));
    app.handle_notepad_history(KeyCode::Char('t'));
    app.handle_notepad_history(KeyCode::Char('a'));
    let preview_before_space = app.play_state.lock().unwrap().clone();

    app.handle_notepad_history(KeyCode::Char(' '));

    assert!(app.notepad_filter_active);
    assert_eq!(app.notepad_query, "beta ");
    assert_eq!(app.notepad_history_items(), vec!["beta soft".to_string()]);
    assert!(*app.play_state.lock().unwrap() == preview_before_space);
}

#[test]
fn handle_notepad_history_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('/'));
    app.handle_notepad_history(KeyCode::Char('p'));
    app.handle_notepad_history(KeyCode::Char('a'));
    app.handle_notepad_history(KeyCode::Char('d'));
    app.handle_notepad_history_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_notepad_history(KeyCode::Char('X'));

    assert!(app.notepad_filter_active);
    assert_eq!(app.notepad_query, "Xpad");
}

#[test]
fn handle_notepad_history_n_p_t_switch_to_corresponding_overlays() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Line Patch"} line phrase"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Line Patch",
        "Pads/Pad 1.fxp",
    ]))));
    app.patch_phrase_store.notepad.history = vec![
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} selected phrase"#.to_string(),
        "plain phrase".to_string(),
    ];
    app.patch_phrase_store.notepad.favorites = vec!["favorite".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["selected phrase".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );
    app.start_notepad_history();

    // overlay 切替キーを統一するため、notepad history 中でも n で先頭選択の初期状態に戻せるようにする。
    app.handle_notepad_history(KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::NotepadHistory));
    assert_eq!(app.notepad_history_cursor, 0);

    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('p'));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Pads/Pad 1.fxp"));

    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('t'));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_filtered[app.patch_cursor], "Pads/Pad 1.fxp");
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
fn handle_notepad_history_starts_scrolling_before_cursor_reaches_view_edge() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec![
        "zero".to_string(),
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
        "four".to_string(),
        "five".to_string(),
        "six".to_string(),
        "seven".to_string(),
    ];
    app.notepad_history_page_size = 6;
    app.start_notepad_history();

    for _ in 0..4 {
        app.handle_notepad_history(KeyCode::Char('j'));
    }
    assert_eq!(app.notepad_history_cursor, 4);
    assert_eq!(app.notepad_history_state.offset(), 1);

    for _ in 0..2 {
        app.handle_notepad_history(KeyCode::Char('k'));
    }
    assert_eq!(app.notepad_history_cursor, 2);
    assert_eq!(app.notepad_history_state.offset(), 0);
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
