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

    let lines = render_lines(&mut app, 80, 10).join("\n");

    assert!(lines.contains("History - Pads/Pad 1.fxp"));
    assert!(lines.contains("Favorites"));
    assert!(lines.contains("l8cdef"));
    assert!(lines.contains("o5g"));
}

#[test]
fn patch_phrase_screen_splits_status_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;

    let lines = render_lines(&mut app, 120, 10);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let normalized_status = "patch phrase".replace(' ', "");
    let status_row = normalized_lines
        .iter()
        .position(|line| line == &normalized_status)
        .unwrap();
    let keybind_row = normalized_lines
        .iter()
        .position(|line| line.contains("j/k・↑↓:再生移動PgUp/PgDn:1画面移動"))
        .unwrap();

    assert_eq!(keybind_row, status_row + 1);
}

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
    assert!(normalized.contains("音色選択-/でpatchname検索"));
    assert!(normalized.contains("パッチ(2/2)"));
    assert!(normalized.contains("Favorite音色(1)"));
    assert!(lines.contains("Pads/Pad 1.fxp"));
    assert!(lines.contains("Leads/Lead 1.fxp"));
}

#[test]
fn patch_select_screen_splits_status_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    let lines = render_lines(&mut app, 160, 16);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let normalized_screen = lines.join("\n").replace([' ', '\n'], "");
    let status_row = normalized_lines
        .iter()
        .position(|line| {
            line.contains("音色選択") && !line.contains("検索") && !line.contains("決定")
        })
        .unwrap();
    let keybind_row = normalized_lines
        .iter()
        .position(|line| line.contains("Enter:検索確定/決定ESC:キャンセル"))
        .unwrap();

    assert!(!normalized_lines[status_row].contains("Enter:決定"));
    assert_eq!(keybind_row, status_row + 1);
    assert!(normalized_lines[keybind_row].contains("/:検索入力"));
    assert!(normalized_lines[keybind_row].contains("n/p/t:overlay切替"));
    assert!(normalized_lines[keybind_row].contains("f:お気に入り"));
    assert!(normalized_screen.contains("h/l・←/→:ペイン移動"));
    assert!(normalized_screen.contains("j/k・↑↓・PgUp/PgDn:移動して再生"));
}

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

    let lines = render_lines(&mut app, 100, 16).join("\n");

    assert!(lines.contains("[HISTORY] notepad mode"));
    assert!(lines.contains("History"));
    assert!(lines.contains("Favorites"));
    assert!(lines.contains("/"));
    assert!(lines.contains("l8cdef"));
    assert!(lines.contains("o5g"));
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
    let (title_x, title_y) = find_text(&buffer, "History");

    assert_eq!(title_y, panes[0].y);
    assert!((panes[0].x..panes[0].x + panes[0].width / 4).contains(&title_x));
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
fn patch_phrase_screen_uses_c_as_fallback_for_empty_lists() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let lines = render_lines(&mut app, 80, 10).join("\n");

    assert!(lines.contains("▶ c"));
}
