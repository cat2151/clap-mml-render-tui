use super::*;

#[test]
fn normal_screen_cursor_uses_contrast_background_without_blink() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.set_session_lines_for_test(vec!["abc".to_string()]);

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
    let mut app = NotepadScreen::new_for_test(test_config());

    assert!(!app.uses_textarea_cursor());

    app.mode = Mode::Insert;
    assert!(app.uses_textarea_cursor());

    app.mode = Mode::PatchSelect;
    app.patch_select.patch_select_filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.patch_select.patch_select_filter_active = false;
    app.mode = Mode::NotepadHistory;
    app.notepad_history.filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.notepad_history.filter_active = false;
    app.mode = Mode::PatchPhrase;
    app.patch_phrase.filter_active = true;
    assert!(app.uses_textarea_cursor());
}
