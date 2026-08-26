use super::*;
use ratatui::{backend::TestBackend, style::Modifier, Terminal};

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
        ('a', PrimaryScreen::DailyDaw),
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
fn daily_daw_serializes_as_a_distinct_primary_screen() {
    let encoded = serde_json::to_string(&PrimaryScreen::DailyDaw).unwrap();

    assert_eq!(encoded, r#""daily_daw""#);
    assert_eq!(
        serde_json::from_str::<PrimaryScreen>(&encoded).unwrap(),
        PrimaryScreen::DailyDaw
    );
    assert!(PrimaryScreen::DailyDaw.is_daw());
    assert!(PrimaryScreen::Daw.is_daw());
    assert!(!PrimaryScreen::Notepad.is_daw());
}

#[test]
fn daily_daw_is_shown_and_highlighted_as_the_current_screen() {
    let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
    terminal
        .draw(|frame| draw_screen_switch_menu(frame, PrimaryScreen::DailyDaw))
        .unwrap();
    let highlighted = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .filter(|cell| cell.fg == MONOKAI_YELLOW && cell.modifier.contains(Modifier::BOLD))
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert_eq!(highlighted, "[A] Daily DAW");
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
