use super::*;

#[test]
fn space_previews_the_current_selection_without_moving() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 3, "Tone 01");
    *app.playback.session.play_state().lock().unwrap() = PlayState::Idle;

    press(&mut app, KeyCode::Char(' '));

    assert_eq!(selected(&app).as_deref(), Some("Tone 01"));
    assert_eq!(
        playing(&app).as_deref(),
        Some(r#"{"Surge XT patch": "Tone 01"} l8cdef"#)
    );
}

#[test]
fn preview_uses_the_line_filter_word_even_after_the_regex_changes() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Pads/Pad 1.fxp","Surge XT patch filter":"pads"} l8cdef"#,
        &["Pads/Pad 1.fxp", "Pads/Pad 2.fxp"],
    );

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, " 2");

    assert_eq!(regex_text(&app), "pads 2");
    assert_eq!(
        playing(&app).as_deref(),
        Some(r#"{"Surge XT patch": "Pads/Pad 2.fxp", "Surge XT patch filter": "pads"} l8cdef"#)
    );
}
