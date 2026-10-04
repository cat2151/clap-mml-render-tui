use std::sync::Arc;

use super::*;

#[test]
fn keyboard_only_route_loads_the_injected_catalog_on_shift_i() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let mut app = TuiApp::new_for_test(test_config());
    app.keyboard.set_chord_progression_source(Arc::new(move || {
        observed.fetch_add(1, Ordering::Relaxed);
        ChordProgressionCatalog::from_json(r#"[{"degrees":"I-V-I"}]"#).unwrap()
    }));
    app.start_keyboard(None);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert!(app.chord_catalog.is_empty());

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));

    assert_eq!(app.active_screen, PrimaryScreen::Keyboard);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(app.keyboard.random_chord_mode());
    assert_eq!(
        app.keyboard.state.note_playback_mode(),
        keyboard::NotePlaybackMode::Off
    );
    let target = app
        .keyboard
        .state
        .repeat_chords()
        .iter()
        .map(|chord| chord.iter().map(|note| note.midi_note).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        cmrt_chord::note_progression(app.keyboard.mml_input.last_confirmed()).unwrap(),
        target
    );
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(
        app.keyboard.mml_input.value(),
        app.keyboard.mml_input.last_confirmed()
    );
}

#[test]
fn keyboard_shared_source_unavailable_keeps_the_current_progression() {
    let mut app = TuiApp::new_for_test(test_config());
    app.keyboard.set_chord_progression_source(
        super::super::keyboard_glue::keyboard_catalog_source_from(
            app.chord_progression_source.clone(),
        ),
    );
    app.start_keyboard(None);
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let saved = app.keyboard.session_state();

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT));

    assert_eq!(app.keyboard.session_state(), saved);
    assert!(!app.keyboard.random_chord_mode());
    assert!(app.keyboard.random_chord_error().is_some());
}
