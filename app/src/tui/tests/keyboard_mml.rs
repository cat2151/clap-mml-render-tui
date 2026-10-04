use super::*;
use crossterm::event::KeyEventKind;

fn press(app: &mut TuiApp<'_>, code: KeyCode) {
    app.handle_keyboard_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

fn enable_random_mode(app: &mut TuiApp<'_>) {
    app.keyboard
        .set_chord_progression_source(std::sync::Arc::new(|| {
            ChordProgressionCatalog::from_json(r#"[{"degrees":"I-IV"}]"#).unwrap()
        }));
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));
    assert!(app.keyboard.random_chord_mode());
}

fn clear_input(app: &mut TuiApp<'_>) {
    press(app, KeyCode::End);
    for _ in 0..app.keyboard.mml_input.value().chars().count() {
        press(app, KeyCode::Backspace);
    }
}

#[test]
fn keyboard_mml_overlay_confirms_progression_and_reopens_with_the_previous_value() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    enable_random_mode(&mut app);

    press(&mut app, KeyCode::Char('i'));
    assert!(app.keyboard.mml_input.is_active());
    clear_input(&mut app);
    for ch in "cec".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);

    assert!(!app.keyboard.mml_input.is_active());
    assert!(!app.keyboard.random_chord_mode());
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
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    enable_random_mode(&mut app);
    let saved = app.keyboard.session_state();

    press(&mut app, KeyCode::Char('i'));
    clear_input(&mut app);
    press(&mut app, KeyCode::Char('r'));
    press(&mut app, KeyCode::Enter);

    assert!(app.keyboard.mml_input.is_active());
    assert_eq!(
        app.keyboard.mml_input.error(),
        Some("MMLに発音ノートがありません")
    );
    assert_eq!(app.keyboard.session_state(), saved);
    assert!(app.keyboard.random_chord_mode());

    press(&mut app, KeyCode::Esc);
    assert!(!app.keyboard.mml_input.is_active());
    assert!(app.keyboard.random_chord_mode());
    assert_eq!(app.keyboard.session_state(), saved);
}

#[test]
fn keyboard_mml_overlay_forwards_physical_note_releases() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    enable_random_mode(&mut app);
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
    assert!(app.keyboard.random_chord_mode());
}

#[test]
fn keyboard_mml_overlay_accepts_key_repeat_for_text_editing() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    enable_random_mode(&mut app);
    press(&mut app, KeyCode::Char('i'));
    clear_input(&mut app);

    app.handle_keyboard_key_event(KeyEvent::new_with_kind(
        KeyCode::Char('c'),
        KeyModifiers::NONE,
        KeyEventKind::Repeat,
    ));

    assert_eq!(app.keyboard.mml_input.value(), "c");
    assert!(app.keyboard.random_chord_mode());
}

#[test]
fn keyboard_overlays_consume_shift_i_before_random_mode_toggle() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    enable_random_mode(&mut app);
    let saved = app.keyboard.session_state();
    press(&mut app, KeyCode::Char('i'));
    let before = app.keyboard.mml_input.value();
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));
    assert_eq!(app.keyboard.mml_input.value(), format!("{before}I"));
    assert!(app.keyboard.random_chord_mode());
    assert_eq!(app.keyboard.session_state(), saved);
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Char('?'));
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));
    assert!(app.keyboard.help_open());
    assert!(app.keyboard.random_chord_mode());
    assert_eq!(app.keyboard.session_state(), saved);
}
