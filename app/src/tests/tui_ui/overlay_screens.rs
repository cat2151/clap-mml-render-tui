use super::*;
use crate::tui::{PatchPhrasePane, PatchSelectPane};

fn pane_contains_cursor_highlight(buffer: &Buffer, pane: ratatui::layout::Rect) -> bool {
    (pane.y..pane.y + pane.height).any(|y| {
        (pane.x..pane.x + pane.width).any(|x| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
        })
    })
}

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

    assert!(status_row > history_row);
    assert!(status_row > favorites_row);
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
    assert!(normalized.contains("ENTERで音色を選択-patchselect-"));
    assert!(normalized.contains("/を押して絞り込み"));
    assert!(normalized.contains("Favorite音色(1)"));
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
    let status_row = keybind_row
        .checked_sub(1)
        .expect("keybind_row must be > 0 so there is a status row above the keybinds");

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
