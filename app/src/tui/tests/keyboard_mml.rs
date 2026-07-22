use super::*;
use crossterm::event::KeyEventKind;

fn press(app: &mut TuiApp<'_>, code: KeyCode) {
    app.handle_keyboard_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn keyboard_mml_overlay_confirms_progression_and_reopens_with_the_previous_value() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;

    press(&mut app, KeyCode::Char('i'));
    assert!(app.keyboard.mml_input.is_active());
    for ch in "cec".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(!app.keyboard.mml_input.is_active());
    assert_eq!(
        app.keyboard
            .state
            .repeat_chords()
            .iter()
            .map(|chord| chord.iter().map(|note| note.midi_note).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![60], vec![64], vec![60]]
    );

    press(&mut app, KeyCode::Char('i'));
    assert_eq!(app.keyboard.mml_input.value(), "cec");
}

#[test]
fn keyboard_mml_error_keeps_the_overlay_and_previous_target() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;
    app.keyboard
        .state
        .replace_repeat_chords(vec![vec![60, 64]], std::time::Instant::now(), false);

    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Char('r'));
    press(&mut app, KeyCode::Enter);

    assert!(app.keyboard.mml_input.is_active());
    assert_eq!(
        app.keyboard.mml_input.error(),
        Some("MMLに発音ノートがありません")
    );
    assert_eq!(
        app.keyboard
            .state
            .repeat_chords()
            .iter()
            .map(|chord| chord.iter().map(|note| note.midi_note).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![60, 64]]
    );

    press(&mut app, KeyCode::Esc);
    assert!(!app.keyboard.mml_input.is_active());
}

#[test]
fn keyboard_mml_overlay_forwards_physical_note_releases() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;
    assert!(app
        .keyboard
        .state
        .press(crate::tui::keyboard::KEYBOARD_NOTES[0])
        .is_some());
    press(&mut app, KeyCode::Char('i'));

    app.handle_keyboard_key_event(KeyEvent::new_with_kind(
        KeyCode::Char('c'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));

    assert!(app.keyboard.state.held().is_empty());
    assert!(app.keyboard.mml_input.is_active());
}

#[test]
fn keyboard_mml_overlay_accepts_key_repeat_for_text_editing() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;
    press(&mut app, KeyCode::Char('i'));

    app.handle_keyboard_key_event(KeyEvent::new_with_kind(
        KeyCode::Char('c'),
        KeyModifiers::NONE,
        KeyEventKind::Repeat,
    ));

    assert_eq!(app.keyboard.mml_input.value(), "c");
}
