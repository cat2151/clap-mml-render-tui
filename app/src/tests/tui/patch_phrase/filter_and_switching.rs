use super::*;

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
fn handle_patch_phrase_filter_ctrl_a_uses_tui_textarea_default_binding() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["alpha".to_string()],
            favorites: vec![],
        },
    );
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('/'));
    app.handle_patch_phrase(KeyCode::Char('p'));
    app.handle_patch_phrase(KeyCode::Char('a'));
    app.handle_patch_phrase(KeyCode::Char('d'));
    app.handle_patch_phrase_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_patch_phrase(KeyCode::Char('X'));

    assert!(app.patch_phrase_filter_active);
    assert_eq!(app.patch_phrase_query, "Xpad");
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
fn handle_patch_phrase_question_mark_enters_help_and_esc_returns_to_patch_phrase() {
    let mut app = TuiApp::new_for_test(test_config());
    app.start_patch_phrase("Pads/Pad 1.fxp".to_string());

    app.handle_patch_phrase(KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::PatchPhrase));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::PatchPhrase));
}
