use super::*;

#[test]
fn draw_shows_solo_and_mute_below_init_meas_during_solo_mode() {
    let mut app = build_test_app();
    app.solo_tracks[1] = true;

    let lines = render_lines(&app, 60, 20);

    assert!(
        lines.iter().any(|line| line.contains("solo")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("mute")),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_grays_out_muted_tracks_during_solo_mode() {
    let mut app = build_test_app();
    app.editor.data[2][1] = "gabc".to_string();
    app.solo_tracks[1] = true;

    let buffer = render_buffer(&app, 60, 20);

    assert_eq!(buffer.cell((1, 6)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((11, 6)).unwrap().fg, MONOKAI_GRAY);
}
