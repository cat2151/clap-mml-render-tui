use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn ctrl_g_is_the_only_screen_switch_trigger() {
    assert!(is_screen_switch_trigger(KeyEvent::new(
        KeyCode::Char('g'),
        KeyModifiers::CONTROL
    )));
    assert!(!is_screen_switch_trigger(key(KeyCode::Char('g'))));
    assert!(!is_screen_switch_trigger(KeyEvent::new(
        KeyCode::Char('g'),
        KeyModifiers::ALT
    )));
}

#[test]
fn menu_accepts_all_screen_initials_case_insensitively() {
    for (input, expected) in [
        ('n', PrimaryScreen::Notepad),
        ('D', PrimaryScreen::Daw),
        ('k', PrimaryScreen::Keyboard),
        ('L', PrimaryScreen::LoopBrowser),
        ('g', PrimaryScreen::GridSequencer),
    ] {
        let mut menu = ScreenSwitchMenu::default();
        menu.open();
        let modifiers = if input.is_ascii_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        assert_eq!(
            menu.handle_key(KeyEvent::new(KeyCode::Char(input), modifiers)),
            ScreenSwitchMenuAction::SwitchTo(expected)
        );
        assert!(!menu.is_open());
    }
}

#[test]
fn escape_closes_menu_and_other_keys_leave_it_open() {
    let mut menu = ScreenSwitchMenu::default();
    menu.open();
    assert_eq!(
        menu.handle_key(key(KeyCode::Char('x'))),
        ScreenSwitchMenuAction::Continue
    );
    assert!(menu.is_open());
    assert_eq!(
        menu.handle_key(key(KeyCode::Esc)),
        ScreenSwitchMenuAction::Closed
    );
    assert!(!menu.is_open());
}
