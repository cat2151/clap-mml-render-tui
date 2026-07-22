use super::*;

#[test]
fn handle_normal_o_and_o_insert_blank_line_and_enter_insert_mode() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('o'));

    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), "line 1".to_string(), String::new()]
    );
    assert_eq!(app.editor.cursor, 2);
    assert_eq!(app.editor.list_state.selected(), Some(2));
    assert!(matches!(app.mode, Mode::Insert));

    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('O'));

    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), String::new(), "line 1".to_string()]
    );
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert!(matches!(app.mode, Mode::Insert));
}
