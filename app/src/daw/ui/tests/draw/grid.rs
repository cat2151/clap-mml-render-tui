use super::*;

#[test]
fn draw_shows_mml_and_uncached_dot_before_cache_is_ready() {
    let mut app = build_test_app();
    app.editor.data[1][1] = "cdef".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Pending;
    }

    let lines = render_lines(&app, 40, 15);

    assert!(
        lines.iter().any(|line| line.contains("cdef")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains('.')),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_renders_pending_indicator_in_visible_color() {
    let mut app = build_test_app();
    app.editor.data[1][1] = "cdef".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Pending;
    }

    let buffer = render_buffer(&app, 40, 15);
    let pending_indicator = (0..buffer.area.height)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.symbol() == "." && cell.fg == MONOKAI_FG
        });

    assert!(
        pending_indicator.is_some(),
        "buffer should contain a visible pending indicator"
    );
}

#[test]
fn draw_uses_contrast_background_for_selected_grid_cell_without_blink() {
    let mut app = build_test_app();
    app.editor.data[0][0] = "t120".to_string();

    let buffer = render_buffer(&app, 40, 14);
    let (x, y) = find_text_ignoring_spaces(&buffer, "t120");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_GRAY);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_GRAY));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}
