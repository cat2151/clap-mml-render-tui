use super::*;

#[test]
fn handle_notepad_history_enter_overwrites_current_line_and_closes() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad.history = vec!["after".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Enter);

    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(app.editor.lines, vec!["after".to_string()]);
    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "after"
    ));
}
