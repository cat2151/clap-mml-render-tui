use super::*;

#[test]
fn handle_insert_ctrl_c_copies_selected_text() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::Insert;
    app.editor.textarea = TextArea::from(["Hello World"]);
    assert_eq!(cmrt_tui_core::clipboard::take_text_for_test(), None);
    app.editor.textarea.move_cursor(CursorMove::WordForward);
    app.editor.textarea.start_selection();
    app.editor.textarea.move_cursor(CursorMove::End);

    app.handle_insert(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(app.editor.textarea.yank_text(), "World");
    assert_eq!(app.editor.textarea.lines().join(""), "Hello World");
    assert_eq!(
        cmrt_tui_core::clipboard::take_text_for_test(),
        Some("World".to_string())
    );
}

#[test]
fn handle_insert_ctrl_x_cuts_selected_text() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::Insert;
    app.editor.textarea = TextArea::from(["Hello World"]);
    app.editor.textarea.move_cursor(CursorMove::WordForward);
    app.editor.textarea.start_selection();
    app.editor.textarea.move_cursor(CursorMove::End);

    app.handle_insert(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));

    assert_eq!(app.editor.textarea.yank_text(), "World");
    assert_eq!(app.editor.textarea.lines().join(""), "Hello ");
}

#[test]
fn handle_insert_ctrl_v_pastes_yanked_text() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.mode = Mode::Insert;
    app.editor.textarea = TextArea::from(["Hello"]);
    app.editor.textarea.move_cursor(CursorMove::End);
    app.editor.textarea.set_yank_text(" World");

    app.handle_insert(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    assert_eq!(app.editor.textarea.lines().join(""), "Hello World");
}
