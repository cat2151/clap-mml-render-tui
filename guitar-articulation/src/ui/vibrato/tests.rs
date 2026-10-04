use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn rendered(screen: &GuitarArticulationScreen) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| crate::ui::draw(screen, f)).unwrap();
    let buffer = terminal.backend().buffer();
    buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

#[test]
fn overlay_shows_four_current_settings_units_and_controls() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('V'), KeyModifiers::SHIFT));
    let initial = rendered(&screen);
    for visible in [
        "Shift+V",
        "待機300ms",
        "立ち上がり400ms",
        "最終深さ64",
        "速度95",
        "CC21・uと共通",
        "j/k",
        "↑↓:選択",
        "h/l",
        "←→:変更",
        "Esc:保持して閉じる",
        "±50ms",
        "±8",
    ] {
        assert!(initial.contains(visible), "{visible} が読めない: {initial}");
    }
    for _ in 0..4 {
        screen.handle_key_event(key(KeyCode::Char('l')));
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    let changed = rendered(&screen);
    for visible in ["待機350ms", "立ち上がり450ms", "最終深さ72", "速度103"] {
        assert!(changed.contains(visible), "{visible} が読めない: {changed}");
    }
    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!rendered(&screen).contains("ビブラート設定"));
}
