use super::*;

#[test]
fn draw_shows_history_overlay_title_and_items() {
    let mut app = build_test_app();
    app.mode = DawMode::History;
    app.history_overlay_patch_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let normalized_lines: Vec<String> = render_lines(&app, 160, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();
    let normalized_screen = normalized_lines.join("").replace(' ', "");
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("patch history - pads/pad 1.fxp")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines.iter().any(|line| line.contains("l8cdef")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("favorites")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_screen.contains("/を押して絞り込み(space=and)"),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_shows_patch_select_overlay_title_and_items() {
    let mut app = build_test_app();
    app.mode = DawMode::PatchSelect;
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
    ];
    app.patch_filtered = app.patch_all.iter().map(|(orig, _)| orig.clone()).collect();
    app.patch_favorite_items = vec!["Pads/Pad 1.fxp".to_string()];

    let normalized_lines: Vec<String> = render_lines(&app, 160, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();
    let normalized_screen = normalized_lines.join("\n").replace([' ', '\n'], "");

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("patch select")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("pads/pad 1.fxp")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("favorite patches")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_screen.contains("/を押して絞り込み"),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_screen.contains("h/l・←/→:ペイン移動してpreview"),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_screen.contains("j/k・↑/↓:移動してpreview"),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_patch_select_shows_filter_input_keybinds_when_filter_active() {
    let mut app = build_test_app();
    app.mode = DawMode::PatchSelect;
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        ("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string()),
    ];
    app.patch_filtered = vec!["Bass/Bass 1.fxp".to_string()];
    app.patch_query = "bass".to_string();
    app.patch_select_filter_active = true;

    let normalized_lines: Vec<String> = render_lines(&app, 140, 30)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("検索入力(Enter=確定/ESC=中断)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Enter:検索確定ESC:検索中断Space:AND条件文字:検索入力")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_patch_select_uses_contrast_cursor_for_filter_input_without_blink() {
    let mut app = build_test_app();
    app.mode = DawMode::PatchSelect;
    app.patch_all = vec![("Bass/Bass 1.fxp".to_string(), "bass/bass 1.fxp".to_string())];
    app.patch_filtered = vec!["Bass/Bass 1.fxp".to_string()];
    app.patch_query = "bass".to_string();
    app.patch_select_filter_active = true;

    let buffer = render_buffer(&app, 140, 30);
    let (_, y) = find_text_ignoring_spaces(&buffer, "bass");
    assert!((0..buffer.area.width).any(|x| {
        let cell = buffer.cell((x, y)).unwrap();
        cell.bg == cursor_highlight_bg(cell.fg)
            && !cell
                .modifier
                .contains(ratatui::style::Modifier::RAPID_BLINK)
    }));
}

#[test]
fn draw_history_overlay_uses_contrast_background_for_selected_entry_without_blink() {
    let mut app = build_test_app();
    app.mode = DawMode::History;
    app.history_overlay_patch_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let buffer = render_buffer(&app, 160, 30);
    let (x, y) = find_text_ignoring_spaces(&buffer, "l8cdef");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}

#[test]
fn insert_and_filter_modes_use_terminal_bar_cursor() {
    let mut app = build_test_app();

    assert!(!app.uses_textarea_cursor());

    app.mode = DawMode::Insert;
    assert!(app.uses_textarea_cursor());

    app.mode = DawMode::History;
    app.history_overlay_filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.history_overlay_filter_active = false;
    app.mode = DawMode::PatchSelect;
    app.patch_select_filter_active = true;
    assert!(app.uses_textarea_cursor());
}
