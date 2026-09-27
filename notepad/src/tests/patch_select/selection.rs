use super::*;

#[test]
fn enter_replaces_the_patch_and_keeps_the_line_filter_word() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"} l8cdef"#,
        &["Pads/Pad 1.fxp", "Pads/Pad 2.fxp", "Leads/Lead 1.fxp"],
    );
    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, " 2");
    press(&mut app, KeyCode::Enter);
    assert_eq!(selected(&app).as_deref(), Some("Pads/Pad 2.fxp"));

    press(&mut app, KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
    assert!(app.patch_select.is_none());
    assert_eq!(
        app.editor.lines,
        vec![
            r#"{"Surge XT patch": "Pads/Pad 2.fxp", "Surge XT patch filter": "pads"} l8cdef"#
                .to_string()
        ]
    );
}

#[test]
fn enter_records_history_and_primes_the_returned_line_into_cache() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 3, "Tone 00");
    press(&mut app, KeyCode::Char('j'));

    press(&mut app, KeyCode::Enter);

    let line = r#"{"Surge XT patch": "Tone 01"} l8cdef"#;
    assert_eq!(app.editor.lines, vec![line.to_string()]);
    assert_eq!(
        app.patch_phrase_store
            .notepad
            .history
            .first()
            .map(String::as_str),
        Some(line)
    );
    assert!(app.audio.cache.lock().unwrap().contains_key(line));
}

#[test]
fn esc_closes_without_touching_the_line() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 3, "Tone 00");
    press(&mut app, KeyCode::Char('j'));

    press(&mut app, KeyCode::Esc);

    assert!(matches!(app.mode, Mode::Normal));
    assert!(app.patch_select.is_none());
    assert_eq!(app.editor.lines, vec![tone_line("Tone 00")]);
}
