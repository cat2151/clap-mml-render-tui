use super::*;

#[test]
fn keyboard_ignores_note_input_until_connection_is_ready() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;

    let result = app.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        KeyCode::Char('c'),
        crossterm::event::KeyModifiers::NONE,
    ));

    assert!(matches!(
        result,
        crate::tui::keyboard::KeyboardAction::Continue
    ));
    assert!(app.keyboard.state.held().is_empty());
}

#[test]
fn keyboard_s_has_no_transport_switch_behavior() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    assert!(app
        .keyboard
        .state
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
    assert_eq!(app.keyboard.state.held().len(), 1);
}

#[test]
fn keyboard_shift_h_cycles_buffer_without_releasing_held_notes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    assert!(app
        .keyboard
        .state
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
    assert_eq!(app.keyboard.state.buffer_multiplier(), 8);
    assert_eq!(app.keyboard.state.held().len(), 1);
}

#[test]
fn keyboard_patch_keys_navigate_categories_and_apply_the_selected_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(make_patches(&[
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
        .keyboard
        .state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 2.fxp")
    );
    assert!(app.keyboard.state.held().is_empty());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 0.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 10.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );
}

#[test]
fn keyboard_vim_keys_and_ctrl_page_keys_navigate_patches() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(make_patches(&[
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
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 2.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 0.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 10.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 0.fxp")
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );
}

#[test]
fn keyboard_r_selects_each_other_patch_once_and_releases_held_notes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(make_patches(&[
        "patches_factory/Lead/Lead 1.fxp",
        "patches_factory/Lead/Lead 2.fxp",
        "patches_factory/Pad/Pad 1.fxp",
    ]))));
    app.start_keyboard(Some("patches_factory/Lead/Lead 1.fxp".to_string()));
    assert!(app
        .keyboard
        .state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    let first = app.keyboard.state.patch().unwrap().to_string();
    assert_ne!(first, "patches_factory/Lead/Lead 1.fxp");
    assert!(app.keyboard.state.held().is_empty());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    let second = app.keyboard.state.patch().unwrap().to_string();
    assert_ne!(second, "patches_factory/Lead/Lead 1.fxp");
    assert_ne!(second, first);

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_ne!(app.keyboard.state.patch(), Some(second.as_str()));
}

#[test]
fn keyboard_keeps_an_unknown_patch_until_the_first_navigation_key() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(make_patches(&[
        "patches_factory/Lead/Lead 1.fxp",
        "patches_factory/Pad/Pad 1.fxp",
    ]))));
    app.start_keyboard(Some("custom/Unknown.fxp".to_string()));

    app.sync_keyboard_patch_catalog();
    assert_eq!(app.keyboard.state.patch(), Some("custom/Unknown.fxp"));
    assert_eq!(
        app.keyboard.state.patch_catalog.selected_category_index(),
        None
    );

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );
}
