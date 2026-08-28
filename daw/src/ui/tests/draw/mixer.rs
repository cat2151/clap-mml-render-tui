use super::*;

#[test]
fn draw_shows_mixer_overlay_with_track_labels_and_db_values() {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.overlays.mixer.cursor_track = 2;
    app.track_volumes_db[2] = -3;
    app.track_volumes_db[3] = 6;

    let normalized_lines: Vec<String> = render_lines(&app, 100, 30)
        .into_iter()
        .map(|line| line.to_lowercase())
        .collect();

    assert!(
        normalized_lines.iter().any(|line| line.contains("mixer")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("track1") && line.contains("track2")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("-3db") && line.contains("+6db")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_highlights_selected_mixer_track_with_contrast_background_without_blink() {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.overlays.mixer.cursor_track = 2;

    let buffer = render_buffer(&app, 100, 30);
    let highlighted_positions: Vec<(u16, u16)> = (0..100)
        .flat_map(|x| (0..30).map(move |y| (x, y)))
        .filter(|(x, y)| {
            let cell = buffer.cell((*x, *y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
                && !cell
                    .modifier
                    .contains(ratatui::style::Modifier::RAPID_BLINK)
        })
        .collect();

    assert!(
        !highlighted_positions.is_empty(),
        "selected mixer track should use a contrast background"
    );

    let (x, y) = find_text_ignoring_spaces(&buffer, "track1");
    let cell = buffer.cell((x, y)).unwrap();
    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
}
