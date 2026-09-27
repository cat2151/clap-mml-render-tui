use super::*;

#[test]
fn f_adds_the_selected_patch_and_line_phrase_to_favorites() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 3, "Tone 00");
    press(&mut app, KeyCode::Char('j'));
    *app.playback.session.play_state().lock().unwrap() = PlayState::Idle;

    press(&mut app, KeyCode::Char('f'));

    let stored = app
        .patch_phrase_store
        .patches
        .get("Tone 01")
        .expect("favorite should be stored for selected patch");
    assert_eq!(stored.favorites, vec!["l8cdef".to_string()]);
    assert!(app.patch_phrase_store_dirty);
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert!(app.patch_select.as_ref().unwrap().is_favorite("Tone 01"));
    assert_eq!(
        playing(&app).as_deref(),
        Some(r#"{"Surge XT patch": "Tone 01"} l8cdef"#)
    );
    choose_favorite_preset(&mut app);
    assert_eq!(patch_select_list(&app), vec!["Tone 01"]);
}

#[test]
fn f_on_the_favorite_preset_puts_the_new_favorite_at_the_top() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.favorite_patches = vec!["Tone 00".to_string()];
    add_favorite(&mut app, "Tone 00", "o5g");
    open_tones(&mut app, 3, "Tone 00");
    choose_favorite_preset(&mut app);
    assert_eq!(patch_select_list(&app), vec!["Tone 00"]);

    // favorite 一覧の外の音色は ALL から選ぶ。
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::End);
    assert_eq!(selected(&app).as_deref(), Some("Tone 02"));
    press(&mut app, KeyCode::Char('f'));
    choose_favorite_preset(&mut app);

    assert_eq!(patch_select_list(&app), vec!["Tone 02", "Tone 00"]);
}

#[test]
fn ctrl_f_also_adds_a_favorite_and_keeps_the_cursor_on_it() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.favorite_patches = vec!["Tone 00".to_string()];
    add_favorite(&mut app, "Tone 00", "o5g");
    open_tones(&mut app, 3, "Tone 00");
    choose_favorite_preset(&mut app);

    press_ctrl(&mut app, 'f');

    assert_eq!(selected(&app).as_deref(), Some("Tone 00"));
    assert_eq!(
        app.patch_phrase_store.patches["Tone 00"].favorites,
        vec!["l8cdef".to_string(), "o5g".to_string()]
    );
}
