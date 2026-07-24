use super::*;
use crate::screen_switch::PrimaryScreen;

fn ctrl_g() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
}

#[test]
fn ctrl_g_opens_menu_on_each_tui_primary_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
    assert!(app.screen_switch_menu.is_open());

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.start_keyboard(None);
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.finish_keyboard();
    app.begin_loop_browser_startup();
    app.loop_browser.state.starting = false;
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
}

#[test]
fn menu_is_unavailable_in_non_normal_states() {
    let mut app = TuiApp::new_for_test(test_config());
    app.notepad.mode = Mode::Insert;
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));

    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.active_screen = PrimaryScreen::LoopBrowser;
    app.loop_browser.state.help_overlay = Some(crate::tui::loop_browser::LoopBrowserPane::Tree);
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
}

#[test]
fn menu_switches_directly_between_tui_primary_screens() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::LoopBrowser, None);
    assert_eq!(app.active_screen, PrimaryScreen::LoopBrowser);
    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::LoopBrowser
    );

    app.loop_browser.state.starting = false;
    app.switch_to_primary_screen(PrimaryScreen::Keyboard, None);
    assert_eq!(app.active_screen, PrimaryScreen::Keyboard);
    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::Keyboard
    );

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
    assert_eq!(app.notepad.mode, Mode::Normal);
}

#[test]
fn selecting_current_screen_only_closes_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    app.screen_switch_menu.open();
    assert_eq!(
        app.handle_screen_switch_menu_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE,)),
        None
    );
    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
    assert!(!app.screen_switch_menu.is_open());
}

#[test]
fn external_screen_switch_closes_an_open_menu() {
    let mut app = TuiApp::new_for_test(test_config());
    app.screen_switch_menu.open();

    app.switch_to_primary_screen(PrimaryScreen::Daw, None);

    assert_eq!(app.active_screen, PrimaryScreen::Daw);
    assert!(!app.screen_switch_menu.is_open());
}
