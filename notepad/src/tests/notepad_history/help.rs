use super::*;

#[test]
fn handle_notepad_history_question_mark_enters_help_and_esc_returns_to_history() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_phrase_store.notepad.history = vec!["alpha".to_string()];
    app.start_notepad_history();

    app.handle_notepad_history(KeyCode::Char('?'));

    assert!(matches!(app.mode, Mode::Help));
    assert!(matches!(app.help_origin, Mode::NotepadHistory));

    app.handle_help(KeyCode::Esc);

    assert!(matches!(app.mode, Mode::NotepadHistory));
}
