use super::*;

#[test]
fn normal_screen_uses_monokai_background_and_border_color() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let buffer = render_buffer(&mut app, 80, 8);

    assert_eq!(buffer.cell((0, 0)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((0, 0)).unwrap().bg, MONOKAI_BG);
    assert_eq!(buffer.cell((4, 4)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((4, 4)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn normal_screen_cursor_uses_contrast_background_without_blink() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let buffer = render_buffer(&mut app, 80, 8);
    let (x, y) = find_text(&buffer, "abc");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}

#[test]
fn insert_and_filter_modes_use_terminal_bar_cursor() {
    let mut app = TuiApp::new_for_test(test_config());

    assert!(!app.uses_textarea_cursor());

    app.mode = Mode::Insert;
    assert!(app.uses_textarea_cursor());

    app.mode = Mode::PatchSelect;
    app.patch_select_filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.patch_select_filter_active = false;
    app.mode = Mode::NotepadHistory;
    app.notepad_filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.notepad_filter_active = false;
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_filter_active = true;
    assert!(app.uses_textarea_cursor());
}

#[test]
fn help_screen_uses_light_gray_escape_hint_on_monokai_background() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Help;

    let buffer = render_buffer(&mut app, 80, 60);
    let (x, y) = find_text(&buffer, "[ESC]");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((x, y)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn status_color_uses_monokai_palette() {
    assert_eq!(status_color(&PlayState::Idle), MONOKAI_CYAN);
    assert_eq!(
        status_color(&PlayState::Running("render".to_string())),
        MONOKAI_PURPLE
    );
    assert_eq!(
        status_color(&PlayState::Playing("play".to_string())),
        MONOKAI_YELLOW
    );
    assert_eq!(
        status_color(&PlayState::Done("done".to_string())),
        MONOKAI_GREEN
    );
    assert_eq!(status_color(&PlayState::Err("err".to_string())), Color::Red);
}

#[test]
fn normal_screen_splits_status_and_keybinds_without_line_numbers() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let lines = render_lines(&mut app, 220, 9);
    let screen = lines.join("\n");
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let status_row = lines
        .iter()
        .position(|line| line.trim_start() == "NORMAL")
        .unwrap();
    let render_row = normalized_lines
        .iter()
        .position(|line| line.contains("並列render中:0"))
        .unwrap();
    let keybind_row = lines
        .iter()
        .position(|line| line.contains("q ?:help i:insert"))
        .unwrap();

    assert!(screen.contains("[NORMAL] notepad mode"));
    assert!(screen.contains("▶ abc"));
    assert!(!screen.contains("MML Lines"));
    assert!(!screen.contains("▶   1 abc"));
    assert_eq!(render_row, status_row + 1);
    assert_eq!(keybind_row, render_row + 1);
    assert!(normalized_lines[render_row].contains("並列render中:0"));
    assert!(screen.contains("q ?:help i:insert"));
    assert!(screen.contains("dd/Del:cut"));
    assert!(screen.contains("g:generate"));
    assert!(screen.contains("Shift+H:patch history"));
    assert!(screen.contains("w:DAW"));
}

#[test]
fn normal_screen_shows_active_parallel_render_count_in_purple() {
    let mut app = TuiApp::new_for_test(test_config());
    app.test_set_active_parallel_render_count(2);

    let buffer = render_buffer(&mut app, 120, 9);
    let (x, y) = find_text(&buffer, "並");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_PURPLE);
}

#[test]
fn insert_screen_shows_insert_title_without_duplicate_line_text() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.start_insert();

    let lines = render_lines(&mut app, 80, 8);
    let screen = lines.join("\n");

    assert!(screen.contains("[INSERT] notepad mode"));
    assert_eq!(screen.matches("abc").count(), 1);
    assert!(lines.iter().any(|line| line.contains("▶ abc")));
}

#[test]
fn patch_phrase_screen_uses_monokai_foreground_for_unfocused_list() {
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

    let buffer = render_buffer(&mut app, 80, 10);
    let (x, y) = find_text(&buffer, "o5g");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_FG);
    assert_eq!(buffer.cell((x, y)).unwrap().bg, MONOKAI_BG);
}
