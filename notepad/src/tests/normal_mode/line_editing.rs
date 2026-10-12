use super::*;

fn press(app: &mut NotepadScreen<'_>, code: KeyCode) -> NormalAction {
    app.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn handle_normal_i_requests_line_edit_without_changing_lines() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    let action = press(&mut app, KeyCode::Char('i'));

    assert!(matches!(action, NormalAction::EditLine));
    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), "line 1".to_string()]
    );
    assert_eq!(app.editor.cursor, 1);
    assert!(matches!(app.mode, Mode::Normal));
    // 依頼は 1 回きり。次のキーまで持ち越さない。
    assert!(matches!(
        press(&mut app, KeyCode::Char('z')),
        NormalAction::Continue
    ));
}

#[test]
fn handle_normal_o_and_o_insert_blank_line_and_request_line_edit() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    let action = press(&mut app, KeyCode::Char('o'));

    assert!(matches!(action, NormalAction::EditLine));
    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), "line 1".to_string(), String::new()]
    );
    assert_eq!(app.editor.cursor, 2);
    assert_eq!(app.editor.list_state.selected(), Some(2));
    assert!(matches!(app.mode, Mode::Normal));

    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    let action = press(&mut app, KeyCode::Char('O'));

    assert!(matches!(action, NormalAction::EditLine));
    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), String::new(), "line 1".to_string()]
    );
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert!(matches!(app.mode, Mode::Normal));
}
