use super::*;

#[test]
fn patch_phrase_screen_renders_history_and_favorites_lists() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let buffer = render_buffer(&mut app, 80, 10);
    let lines = render_lines(&mut app, 80, 10).join("\n");
    let normalized = lines.replace(' ', "");
    let overlay_area = crate::ui_utils::centered_rect(88, 84, buffer.area);
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
    let (_, title_y) = find_text_ignoring_spaces(&buffer, "フレーズ選択");

    assert_eq!(title_y, panes[0].y);
    assert!(lines.contains("Favorites"));
    assert!(lines.contains("l8cdef"));
    assert!(lines.contains("o5g"));
    assert!(normalized.contains("/を押して絞り込み(space=AND)"));
    assert!(normalized.contains("ENTERでフレーズを選択-patchphrasehistory-"));
    assert!(normalized.contains("patchphrase現在1行目/全1行(1/1)"));
}

#[test]
fn patch_phrase_screen_renders_as_overlay_on_notepad_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let buffer = render_buffer(&mut app, 100, 16);
    let lines = render_lines(&mut app, 100, 16).join("\n");
    let overlay_area = crate::ui_utils::centered_rect(88, 84, buffer.area);
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
    let (_, title_y) = find_text_ignoring_spaces(&buffer, "フレーズ選択");

    assert!(lines.contains("[PATCH PHRASE] notepad mode"));
    assert_eq!(title_y, panes[0].y);
    assert!(lines.contains("Favorites"));
}

#[test]
fn patch_phrase_overlay_is_centered_like_other_overlays() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} old"#.to_string()];
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let buffer = render_buffer(&mut app, 100, 20);
    let overlay_area = crate::ui_utils::centered_rect(88, 84, buffer.area);
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
    let (title_x, title_y) = find_text_ignoring_spaces(&buffer, "フレーズ選択");

    assert_eq!(title_y, panes[0].y);
    assert!((panes[0].x..panes[0].x + panes[0].width / 2).contains(&title_x));
    assert!(overlay_area.x > 0);
    assert!(overlay_area.y > 0);
}

#[test]
fn patch_phrase_screen_keeps_status_below_overlay_panes() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.test_set_active_parallel_render_count(2);
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let lines = render_lines(&mut app, 220, 16);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let normalized_status = "patch phrase".replace(' ', "");
    let buffer = render_buffer(&mut app, 220, 16);
    let (_, history_row) = find_text_ignoring_spaces(&buffer, "フレーズ選択");
    let (_, favorites_row) = find_text(&buffer, "Favorites");
    let status_row = normalized_lines
        .iter()
        .rposition(|line| line.contains(&normalized_status))
        .unwrap() as u16;
    let render_row = normalized_lines
        .iter()
        .rposition(|line| line.contains("render:実行2/2予約0"))
        .unwrap() as u16;

    assert!(status_row > history_row);
    assert!(status_row > favorites_row);
    assert_eq!(render_row, status_row + 1);
}

#[test]
fn patch_phrase_screen_shows_filter_confirm_title_when_filter_active() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_filter_active = true;
    app.patch_phrase_query = "l8".to_string();

    let normalized = render_lines(&mut app, 100, 16).join("\n").replace(' ', "");

    assert!(normalized.contains("ENTERで絞り込みを決定-patchphrasehistory-"));
    assert!(
        normalized.match_indices("l8").count() >= 2,
        "expected the active filter query to be rendered in addition to the history entry: {normalized}"
    );
}

#[test]
fn patch_phrase_overlay_marks_cached_preview_items_with_music_note() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.audio_cache.lock().unwrap().insert(
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string(),
        vec![0.1, 0.2],
    );

    let screen = render_lines(&mut app, 100, 16).join("\n");

    assert!(screen.contains("♪ l8cdef"));
    assert!(screen.contains("  o5g"));
}

#[test]
fn patch_phrase_selected_entry_uses_contrast_background_without_blink() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let buffer = render_buffer(&mut app, 100, 16);
    let (x, y) = find_text(&buffer, "l8cdef");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}

#[test]
fn patch_phrase_only_highlights_the_focused_pane() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));
    app.patch_phrase_focus = PatchPhrasePane::History;

    let buffer = render_buffer(&mut app, 100, 16);
    let overlay_area = crate::ui_utils::centered_rect(88, 84, buffer.area);
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
fn patch_phrase_screen_uses_c_as_fallback_for_empty_lists() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let lines = render_lines(&mut app, 80, 10).join("\n");

    assert!(lines.contains("▶   c"));
}
