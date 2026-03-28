use super::*;

#[test]
fn handle_insert_ctrl_c_copies_selected_text() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Insert;
    app.textarea = TextArea::from(["Hello World"]);
    assert_eq!(crate::clipboard::take_text_for_test(), None);
    app.textarea.move_cursor(CursorMove::WordForward);
    app.textarea.start_selection();
    app.textarea.move_cursor(CursorMove::End);

    app.handle_insert(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    assert_eq!(app.textarea.yank_text(), "World");
    assert_eq!(app.textarea.lines().join(""), "Hello World");
    assert_eq!(
        crate::clipboard::take_text_for_test(),
        Some("World".to_string())
    );
}

#[test]
fn handle_insert_ctrl_x_cuts_selected_text() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Insert;
    app.textarea = TextArea::from(["Hello World"]);
    app.textarea.move_cursor(CursorMove::WordForward);
    app.textarea.start_selection();
    app.textarea.move_cursor(CursorMove::End);

    app.handle_insert(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));

    assert_eq!(app.textarea.yank_text(), "World");
    assert_eq!(app.textarea.lines().join(""), "Hello ");
}

#[test]
fn handle_insert_ctrl_v_pastes_yanked_text() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Insert;
    app.textarea = TextArea::from(["Hello"]);
    app.textarea.move_cursor(CursorMove::End);
    app.textarea.set_yank_text(" World");

    app.handle_insert(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    assert_eq!(app.textarea.lines().join(""), "Hello World");
}
