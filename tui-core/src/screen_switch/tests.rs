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
        ('C', PrimaryScreen::ChordChart),
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

/// セッションの `active_screen` はこの綴りでファイルへ入る。変えると
/// 「前回 chord chart で終了した」復元が黙って notepad へ落ちる。
#[test]
fn chord_chart_serializes_as_a_distinct_primary_screen() {
    let encoded = serde_json::to_string(&PrimaryScreen::ChordChart).unwrap();

    assert_eq!(encoded, r#""chord_chart""#);
    assert_eq!(
        serde_json::from_str::<PrimaryScreen>(&encoded).unwrap(),
        PrimaryScreen::ChordChart
    );
    assert!(!PrimaryScreen::ChordChart.is_daw());
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

/// 画面が増えても menu の行が枠から溢れないこと（人間の入口はこの 1 行だけ）。
#[test]
fn the_menu_lists_every_screen_including_the_chord_chart() {
    let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
    terminal
        .draw(|frame| draw_screen_switch_menu(frame, PrimaryScreen::Notepad))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let rendered = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    for label in [
        "[N] Notepad",
        "[A] Daily DAW",
        "[D] DAW",
        "[K] Keyboard",
        "[L] Loop Browser",
        "[G] Grid Sequencer",
        "[C] Chord Chart",
    ] {
        assert!(
            rendered.contains(label),
            "{label} missing from:
{rendered}"
        );
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
