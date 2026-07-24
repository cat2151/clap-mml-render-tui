use super::*;

#[test]
fn handle_normal_dd_yanks_current_line_and_keeps_notepad_non_empty() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    let result = app.handle_normal(KeyCode::Char('d'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(app.editor.pending_delete);
    assert!(app.editor.yank_buffer.is_none());

    let result = app.handle_normal(KeyCode::Char('d'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), "line 2".to_string()]
    );
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert!(!app.editor.pending_delete);
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("line 1"));
}

#[test]
fn handle_normal_d_is_cleared_when_another_key_is_pressed() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 0;
    app.editor.list_state.select(Some(0));

    app.handle_normal(KeyCode::Char('d'));
    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), "line 1".to_string()]
    );
    assert!(!app.editor.pending_delete);
    assert!(app.editor.yank_buffer.is_none());
}

#[test]
fn handle_normal_dd_on_single_line_replaces_with_empty() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["only".to_string()];

    app.handle_normal(KeyCode::Char('d'));
    app.handle_normal(KeyCode::Char('d'));

    assert_eq!(app.editor.lines, vec![String::new()]);
    assert_eq!(app.editor.cursor, 0);
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("only"));
}

#[test]
fn handle_normal_delete_yanks_current_line_and_keeps_notepad_non_empty() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));

    app.handle_normal(KeyCode::Delete);

    assert_eq!(
        app.editor.lines,
        vec!["line 0".to_string(), "line 2".to_string()]
    );
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("line 1"));

    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["only".to_string()];

    app.handle_normal(KeyCode::Delete);

    assert_eq!(app.editor.lines, vec![String::new()]);
    assert_eq!(app.editor.cursor, 0);
    assert_eq!(app.editor.yank_buffer.as_deref(), Some("only"));
}

#[test]
fn handle_normal_p_and_p_paste_yanked_line_below_or_above_cursor() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 0;
    app.editor.list_state.select(Some(0));
    app.editor.yank_buffer = Some("yanked".to_string());

    app.handle_normal(KeyCode::Char('p'));

    assert_eq!(
        app.editor.lines,
        vec![
            "line 0".to_string(),
            "yanked".to_string(),
            "line 1".to_string()
        ]
    );
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));

    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.editor.cursor = 1;
    app.editor.list_state.select(Some(1));
    app.editor.yank_buffer = Some("yanked".to_string());

    app.handle_normal(KeyCode::Char('P'));

    assert_eq!(
        app.editor.lines,
        vec![
            "line 0".to_string(),
            "yanked".to_string(),
            "line 1".to_string()
        ]
    );
    assert_eq!(app.editor.cursor, 1);
    assert_eq!(app.editor.list_state.selected(), Some(1));
}

#[test]
fn handle_normal_p_shows_error_when_yank_buffer_is_empty() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('p'));

    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(msg) if msg == "yank バッファが空です"
    ));
    assert_eq!(app.editor.lines, vec!["cde".to_string()]);
}
