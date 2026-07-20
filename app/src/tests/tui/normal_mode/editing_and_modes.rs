use super::*;

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('?'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Help));
}

#[test]
fn handle_normal_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = (0..8).map(|i| format!("line {i}")).collect();
    app.normal_page_size = 3;
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::PageDown);
    assert_eq!(app.cursor, 4);
    assert_eq!(app.list_state.selected(), Some(4));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 4"
    ));

    app.handle_normal(KeyCode::PageUp);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 1"
    ));
}

#[test]
fn handle_normal_j_prefetches_predicted_navigation_cache() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    app.normal_page_size = 2;

    app.handle_normal(KeyCode::Char('j'));

    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key("line 0"));
    assert!(cache.contains_key("line 2"));
    assert!(cache.contains_key("line 3"));
}

#[test]
fn handle_normal_j_prefetches_direction_first_then_fills_remaining_navigation_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = (0..12).map(|i| format!("line {i}")).collect();
    app.cursor = 4;
    app.normal_page_size = 5;
    app.list_state.select(Some(4));

    app.handle_normal(KeyCode::Char('j'));

    assert_eq!(
        app.audio_cache_order
            .lock()
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["line 6", "line 7", "line 4", "line 10", "line 0", "line 8", "line 9",]
    );
}

#[test]
fn handle_normal_f_shows_error_when_current_line_has_no_patch_json() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patch name JSON が見つかりません"
    ));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_p_shows_error_when_yank_buffer_is_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('p'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "yank バッファが空です"
    ));
    assert_eq!(app.lines, vec!["cde".to_string()]);
}

#[test]
fn handle_normal_f_enters_patch_phrase_for_current_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} cde"#.to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Pads/Pad 1.fxp"));
    assert_eq!(app.patch_phrase_history_items(), vec!["c".to_string()]);
    assert_eq!(app.patch_phrase_favorite_items(), vec!["c".to_string()]);
}

#[test]
fn handle_normal_o_and_o_insert_blank_line_and_enter_insert_mode() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('o'));

    assert_eq!(
        app.lines,
        vec!["line 0".to_string(), "line 1".to_string(), String::new()]
    );
    assert_eq!(app.cursor, 2);
    assert_eq!(app.list_state.selected(), Some(2));
    assert!(matches!(app.mode, Mode::Insert));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('O'));

    assert_eq!(
        app.lines,
        vec!["line 0".to_string(), String::new(), "line 1".to_string()]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(app.mode, Mode::Insert));
}

#[test]
fn handle_normal_w_launches_daw() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_v_launches_keyboard() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('v'));

    assert!(matches!(result, NormalAction::LaunchKeyboard));
}

#[test]
fn start_keyboard_from_notepad_uses_current_cursor_line_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        r#"{"Surge XT patch":"Pads/First.fxp"} c"#.to_string(),
        r#"{"Surge XT patch":"Keys/Current.fxp"} d"#.to_string(),
        r#"{"Surge XT patch":"Leads/Last.fxp"} e"#.to_string(),
    ];
    app.cursor = 1;

    app.start_keyboard_from_notepad();

    assert!(matches!(app.mode, Mode::Keyboard));
    assert_eq!(app.keyboard_state.patch(), Some("Keys/Current.fxp"));
}

#[test]
fn start_keyboard_from_notepad_uses_init_saw_without_valid_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":""} c"#.to_string()];

    app.start_keyboard_from_notepad();

    assert!(matches!(app.mode, Mode::Keyboard));
    assert_eq!(app.keyboard_state.patch(), None);
}

#[test]
fn keyboard_ignores_note_input_until_connection_is_ready() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;

    let result = app.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        KeyCode::Char('c'),
        crossterm::event::KeyModifiers::NONE,
    ));

    assert!(matches!(
        result,
        crate::tui::keyboard::KeyboardAction::Continue
    ));
    assert!(app.keyboard_state.held().is_empty());
}

#[test]
fn keyboard_s_clears_held_notes_before_transport_switch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;
    assert!(app
        .keyboard_state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());

    let result = app.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        KeyCode::Char('s'),
        crossterm::event::KeyModifiers::NONE,
    ));

    assert!(matches!(
        result,
        crate::tui::keyboard::KeyboardAction::Continue
    ));
    assert!(app.keyboard_state.held().is_empty());
}

#[test]
fn keyboard_shift_h_cycles_buffer_without_releasing_held_notes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;
    assert!(app
        .keyboard_state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());

    let result = app.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        KeyCode::Char('H'),
        crossterm::event::KeyModifiers::SHIFT,
    ));

    assert!(matches!(
        result,
        crate::tui::keyboard::KeyboardAction::Continue
    ));
    assert_eq!(app.keyboard_state.buffer_multiplier(), 8);
    assert_eq!(app.keyboard_state.held().len(), 1);
}

#[test]
fn keyboard_patch_keys_navigate_categories_and_apply_the_selected_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Lead/Lead 1.fxp",
        "patches_factory/Lead/Lead 2.fxp",
        "patches_factory/Pad/Pad 0.fxp",
        "patches_factory/Pad/Pad 1.fxp",
        "patches_factory/Pad/Pad 2.fxp",
        "patches_factory/Pad/Pad 3.fxp",
        "patches_factory/Pad/Pad 4.fxp",
        "patches_factory/Pad/Pad 5.fxp",
        "patches_factory/Pad/Pad 6.fxp",
        "patches_factory/Pad/Pad 7.fxp",
        "patches_factory/Pad/Pad 8.fxp",
        "patches_factory/Pad/Pad 9.fxp",
        "patches_factory/Pad/Pad 10.fxp",
        "patches_factory/Pad/Pad 11.fxp",
    ]))));
    app.start_keyboard(Some("patches_factory/Lead/Lead 1.fxp".to_string()));
    assert!(app
        .keyboard_state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Lead/Lead 2.fxp")
    );
    assert!(app.keyboard_state.held().is_empty());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Pad/Pad 0.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Pad/Pad 10.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );
}

#[test]
fn keyboard_vim_keys_and_ctrl_page_keys_navigate_patches() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Lead/Lead 1.fxp",
        "patches_factory/Lead/Lead 2.fxp",
        "patches_factory/Pad/Pad 0.fxp",
        "patches_factory/Pad/Pad 1.fxp",
        "patches_factory/Pad/Pad 2.fxp",
        "patches_factory/Pad/Pad 3.fxp",
        "patches_factory/Pad/Pad 4.fxp",
        "patches_factory/Pad/Pad 5.fxp",
        "patches_factory/Pad/Pad 6.fxp",
        "patches_factory/Pad/Pad 7.fxp",
        "patches_factory/Pad/Pad 8.fxp",
        "patches_factory/Pad/Pad 9.fxp",
        "patches_factory/Pad/Pad 10.fxp",
    ]))));
    app.start_keyboard(Some("patches_factory/Lead/Lead 1.fxp".to_string()));

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Lead/Lead 2.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Pad/Pad 0.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Pad/Pad 10.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Pad/Pad 0.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );
}

#[test]
fn keyboard_r_selects_each_other_patch_once_and_releases_held_notes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Lead/Lead 1.fxp",
        "patches_factory/Lead/Lead 2.fxp",
        "patches_factory/Pad/Pad 1.fxp",
    ]))));
    app.start_keyboard(Some("patches_factory/Lead/Lead 1.fxp".to_string()));
    assert!(app
        .keyboard_state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    let first = app.keyboard_state.patch().unwrap().to_string();
    assert_ne!(first, "patches_factory/Lead/Lead 1.fxp");
    assert!(app.keyboard_state.held().is_empty());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    let second = app.keyboard_state.patch().unwrap().to_string();
    assert_ne!(second, "patches_factory/Lead/Lead 1.fxp");
    assert_ne!(second, first);

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_ne!(app.keyboard_state.patch(), Some(second.as_str()));
}

#[test]
fn keyboard_keeps_an_unknown_patch_until_the_first_navigation_key() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "patches_factory/Lead/Lead 1.fxp",
        "patches_factory/Pad/Pad 1.fxp",
    ]))));
    app.start_keyboard(Some("custom/Unknown.fxp".to_string()));

    app.sync_keyboard_patch_catalog();
    assert_eq!(app.keyboard_state.patch(), Some("custom/Unknown.fxp"));
    assert_eq!(
        app.keyboard_state.patch_catalog.selected_category_index(),
        None
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard_state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );
}

#[test]
fn handle_normal_e_requests_config_edit() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('e'));

    assert!(matches!(result, NormalAction::EditConfig));
    assert!(matches!(app.mode, Mode::Normal));
}
