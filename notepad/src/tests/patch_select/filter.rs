use super::*;

#[test]
fn parent_dir_supplies_the_default_generic_display_path_query() {
    assert_eq!(
        NotepadScreen::default_display_path_filter_query_from_parent_dir(
            "patches_factory/Instrument/Soft Strum.fxp"
        )
        .as_deref(),
        Some("instrument")
    );
    assert_eq!(
        NotepadScreen::default_display_path_filter_query_from_parent_dir(
            "patches_3rdparty/Acme/Guitars/Clean Voice.fxp"
        )
        .as_deref(),
        Some("guitars")
    );
    assert_eq!(
        NotepadScreen::default_display_path_filter_query_from_parent_dir("No Category.fxp"),
        None
    );
}

#[test]
fn slash_then_chars_filter_and_preview_the_first_result() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 12, "Tone 00");

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "1");

    assert_eq!(
        patch_select_list(&app),
        vec!["Tone 01", "Tone 10", "Tone 11"]
    );
    assert_eq!(selected(&app).as_deref(), Some("Tone 01"));
    assert_eq!(
        playing(&app).as_deref(),
        Some(r#"{"Surge XT patch": "Tone 01"} l8cdef"#)
    );
}

#[test]
fn enter_ends_editing_and_keeps_the_filtered_results() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 12, "Tone 00");

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "1");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('j'));

    assert!(!app.patch_select.as_ref().unwrap().filter_editing());
    assert_eq!(regex_text(&app), "1");
    assert_eq!(selected(&app).as_deref(), Some("Tone 10"));
}

#[test]
fn backspace_on_an_empty_regex_keeps_editing() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 2, "Tone 00");

    press(&mut app, KeyCode::Char('/'));
    press(&mut app, KeyCode::Backspace);

    assert!(app.patch_select.as_ref().unwrap().filter_editing());
    assert_eq!(regex_text(&app), "");
}

#[test]
fn regex_editing_uses_the_textarea_default_bindings() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 2, "Tone 00");

    press(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "pad");
    press_ctrl(&mut app, 'a');
    type_text(&mut app, "X");

    assert!(app.patch_select.as_ref().unwrap().filter_editing());
    assert_eq!(regex_text(&app), "Xpad");
}

#[test]
fn opening_prefills_the_regex_with_the_line_filter_word() {
    let mut app = NotepadScreen::new_for_test(test_config());

    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Tone 02","Surge XT patch filter":"0[12]"} l8cdef"#,
        &["Tone 00", "Tone 01", "Tone 02"],
    );

    assert_eq!(regex_text(&app), "0[12]");
    assert!(!app.patch_select.as_ref().unwrap().filter_editing());
    assert_eq!(patch_select_list(&app), vec!["Tone 01", "Tone 02"]);
    assert_eq!(selected(&app).as_deref(), Some("Tone 02"));
}

#[test]
fn an_invalid_line_filter_word_still_opens_with_an_empty_list() {
    let mut app = NotepadScreen::new_for_test(test_config());

    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Tone 00","Surge XT patch filter":"["} l8cdef"#,
        &["Tone 00", "Tone 01"],
    );

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(regex_text(&app), "[");
    assert!(patch_select_list(&app).is_empty());
}
