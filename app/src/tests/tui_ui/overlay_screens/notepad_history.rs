use super::*;

#[test]
fn notepad_history_overlay_renders_history_and_favorites_lists() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();
    app.notepad_filter_active = true;

    let buffer = render_buffer(&mut app, 100, 16);
    let lines = render_lines(&mut app, 100, 16).join("\n");
    let normalized = lines.replace(' ', "");
    let overlay_area = crate::ui_utils::centered_rect(88, 76, buffer.area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(overlay_area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let (_, title_y) = find_text_ignoring_spaces(&buffer, "音色&フレーズ選択");

    assert!(lines.contains("[HISTORY] notepad mode"));
    assert_eq!(title_y, panes[0].y);
    assert!(lines.contains("Favorites"));
    assert!(lines.contains("/"));
    assert!(lines.contains("l8cdef"));
    assert!(lines.contains("o5g"));
    assert!(normalized.contains("ENTERで絞り込みを決定-notepadhistory-"));
    assert!(normalized.contains("notepadhistory現在1行目/全1行(1/1)"));
}

#[test]
fn notepad_history_overlay_shows_selection_title_when_filter_inactive() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();

    let normalized = render_lines(&mut app, 100, 16).join("\n").replace(' ', "");

    assert!(normalized.contains("ENTERで音色とフレーズを選択-notepadhistory-"));
    assert!(normalized.contains("/を押して絞り込み(space=AND)"));
}

#[test]
fn notepad_history_overlay_marks_cached_items_with_music_note() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.audio_cache
        .lock()
        .unwrap()
        .insert("l8cdef".to_string(), vec![0.1, 0.2]);
    app.start_notepad_history();

    let screen = render_lines(&mut app, 100, 16).join("\n");

    assert!(screen.contains("♪ l8cdef"));
    assert!(screen.contains("  o5g"));
}

#[test]
fn notepad_history_overlay_is_centered_like_daw_overlay() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();

    let buffer = render_buffer(&mut app, 100, 20);
    let overlay_area = crate::ui_utils::centered_rect(88, 76, buffer.area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(overlay_area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let (title_x, title_y) = find_text_ignoring_spaces(&buffer, "音色&フレーズ選択");

    assert_eq!(title_y, panes[0].y);
    assert!((panes[0].x..panes[0].x + panes[0].width / 2).contains(&title_x));
    assert!(overlay_area.x > 0);
    assert!(overlay_area.y > 0);
    assert!(buffer.content().iter().any(|cell| cell.symbol() == "▶"));
}

#[test]
fn notepad_history_overlay_shows_left_right_pane_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();

    let lines = render_lines(&mut app, 120, 16);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("h/l・←/→:ペイン移動")));
}

#[test]
fn notepad_history_only_highlights_the_focused_pane() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();
    app.notepad_history_state.select(Some(0));
    app.notepad_favorites_state.select(Some(0));
    app.notepad_focus = PatchPhrasePane::History;

    let buffer = render_buffer(&mut app, 100, 16);
    let overlay_area = crate::ui_utils::centered_rect(88, 76, buffer.area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(overlay_area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    assert!(pane_contains_cursor_highlight(&buffer, panes[0]));
    assert!(!pane_contains_cursor_highlight(&buffer, panes[1]));
}

#[test]
fn notepad_history_guide_overlay_renders_centered_message() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["plain phrase".to_string()];
    app.mode = Mode::NotepadHistoryGuide;

    let buffer = render_buffer(&mut app, 100, 16);
    let normalized = render_lines(&mut app, 100, 16)
        .join("\n")
        .replace([' ', '\n'], "");
    let overlay_area = crate::ui_utils::centered_rect(56, 36, buffer.area);
    let guide_message = "現在の行にはpatch nameがありません。";
    let (text_x, text_y) = find_text_ignoring_spaces(&buffer, &guide_message.replace(' ', ""));

    assert!(normalized.contains("▶plainphrase"));
    assert!(normalized.contains("notepadhistoryoverlayを開きます。"));
    assert!(normalized.contains("ENTERを押してください"));
    assert_eq!(text_y, overlay_area.y + 1);
    assert!((overlay_area.x..overlay_area.x + overlay_area.width).contains(&text_x));
}

#[test]
fn notepad_history_guide_overlay_shows_guide_footer_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["plain phrase".to_string()];
    app.mode = Mode::NotepadHistoryGuide;

    let normalized = render_lines(&mut app, 180, 16)
        .join("\n")
        .replace([' ', '\n'], "");

    assert!(normalized.contains("[NORMAL]notepadmode"));
    assert!(normalized.contains("Enter:notepadhistoryoverlayESC:キャンセル"));
    assert!(!normalized.contains("q?:helpi:inserto/O:挿入"));
}
