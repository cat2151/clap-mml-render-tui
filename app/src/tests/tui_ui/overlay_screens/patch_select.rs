use super::*;

#[test]
fn patch_select_screen_renders_as_overlay_on_normal_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} abc"#.to_string()];
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        (
            "Leads/Lead 1.fxp".to_string(),
            "leads/lead 1.fxp".to_string(),
        ),
    ];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_phrase_store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        PatchPhraseState {
            history: vec![],
            favorites: vec!["abc".to_string()],
        },
    );
    app.patch_favorite_items = vec!["Leads/Lead 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    let lines = render_lines(&mut app, 80, 16).join("\n");
    let normalized = lines.replace(' ', "");

    assert!(lines.contains("[PATCH SELECT] notepad mode"));
    assert!(lines.contains("▶ {\"Surge XT patch\":\"Pads/Pad 1.fxp\"} abc"));
    assert!(normalized.contains("ENTERで音色を選択-patchselect-"));
    assert!(normalized.contains("/を押して絞り込み"));
    assert!(normalized.contains("Favorite音色(1)"));
    assert!(normalized.contains("音色選択現在1行目/全2行(1/2)"));
    assert!(lines.contains("Pads/Pad 1.fxp"));
    assert!(lines.contains("Leads/Lead 1.fxp"));
}

#[test]
fn patch_select_screen_shows_filter_confirm_title_when_filter_active() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        (
            "Leads/Lead 1.fxp".to_string(),
            "leads/lead 1.fxp".to_string(),
        ),
    ];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;
    app.patch_select_filter_active = true;
    app.patch_query = "pad".to_string();

    let normalized = render_lines(&mut app, 100, 16).join("\n").replace(' ', "");

    assert!(normalized.contains("ENTERで絞り込みを決定-patchselect-"));
    assert!(normalized.contains("pad"));
}

#[test]
fn patch_select_screen_splits_status_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.test_set_active_parallel_render_count(2);
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    let lines = render_lines(&mut app, 160, 16);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let normalized_screen = lines.join("\n").replace([' ', '\n'], "");
    let keybind_row = normalized_lines
        .iter()
        .position(|line| line.contains("Enter:検索確定/決定ESC:キャンセル"))
        .unwrap();
    let render_row = keybind_row
        .checked_sub(1)
        .expect("keybind_row must be > 0 so there is a render row above the keybinds");
    let status_row = render_row
        .checked_sub(1)
        .expect("render_row must be > 0 so there is a status row above the render row");

    assert!(!normalized_lines[status_row].contains("Enter:決定"));
    assert_eq!(render_row, status_row + 1);
    assert_eq!(keybind_row, render_row + 1);
    assert!(normalized_lines[status_row].contains("sort:path"));
    assert!(normalized_lines[render_row].contains("並列render中:2"));
    assert!(normalized_lines[keybind_row].contains("/:検索入力"));
    assert!(normalized_lines[keybind_row].contains("Ctrl+S:sort順切替"));
    assert!(normalized_lines[keybind_row].contains("n/p/t:overlay切替"));
    assert!(normalized_lines[keybind_row].contains("f:お気に入り"));
    assert!(normalized_screen.contains("h/l・←/→:ペイン移動"));
    assert!(normalized_screen.contains("j/k・↑↓・PgUp/PgDn:移動して再生"));
}

#[test]
fn patch_select_filter_uses_query_cursor_only() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;
    app.patch_select_filter_active = true;
    app.patch_query = "pad".to_string();

    let buffer = render_buffer(&mut app, 100, 16);
    let cursor = render_cursor_position(&mut app, 100, 16);
    let overlay_area = crate::ui_utils::centered_rect(82, 70, buffer.area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(overlay_area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let query_inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(chunks[0]);

    assert_eq!(cursor.y, query_inner.y);
    assert!((query_inner.x..query_inner.x + query_inner.width).contains(&cursor.x));
    assert!(!(panes[0].y..panes[0].y + panes[0].height).any(|y| {
        (panes[0].x..panes[0].x + panes[0].width).any(|x| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
        })
    }));
    assert!(!(panes[1].y..panes[1].y + panes[1].height).any(|y| {
        (panes[1].x..panes[1].x + panes[1].width).any(|x| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
        })
    }));
}

#[test]
fn patch_select_only_highlights_the_focused_pane() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
    ];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Bass/Bass 1.fxp".to_string()];
    app.patch_favorite_items = vec!["Bass/Bass 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.patch_favorites_state.select(Some(0));
    app.patch_select_focus = PatchSelectPane::Patches;
    app.mode = Mode::PatchSelect;

    let buffer = render_buffer(&mut app, 100, 16);
    let overlay_area = crate::ui_utils::centered_rect(82, 70, buffer.area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
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
