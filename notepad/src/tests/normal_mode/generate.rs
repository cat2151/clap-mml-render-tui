use super::*;

#[test]
fn handle_normal_g_inserts_generated_line_above_current_line_and_plays_it() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('g'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert_eq!(app.editor.lines[0], "line 0");
    assert_eq!(app.editor.lines[2], "line 1");
    let inserted = &app.editor.lines[1];
    assert!(
        inserted == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} c1"#
            || inserted == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cfg1"#,
        "inserted: {inserted}"
    );
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![inserted.clone()]
    );
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pads/Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec![inserted
            .strip_prefix(r#"{"Surge XT patch": "Pads/Pad 1.fxp"} "#)
            .unwrap()
            .to_string()])
    );
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == inserted
    ));
}

#[test]
fn handle_normal_g_shows_error_when_patches_are_unavailable() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new())));

    let result = app.handle_normal(KeyCode::Char('g'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dirs にパッチが見つかりません"
    ));
}
