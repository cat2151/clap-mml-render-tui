use super::*;

#[test]
fn handle_normal_f_shows_error_when_current_line_has_no_patch_json() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(
        &*app.playback.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patch name JSON が見つかりません"
    ));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_f_enters_patch_phrase_for_current_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} cde"#.to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(
        app.patch_phrase.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert_eq!(app.patch_phrase_history_items(), vec!["c".to_string()]);
    assert_eq!(app.patch_phrase_favorite_items(), vec!["c".to_string()]);
}
