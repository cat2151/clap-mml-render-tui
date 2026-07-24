use super::*;

#[test]
fn draw_shows_log_pane_with_all_borders() {
    let app = build_test_app();

    let lines = render_lines(&app, 60, 10);

    assert!(
        lines.iter().any(|line| line.contains("┌ log ")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("└")),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_shows_log_pane_in_lower_half() {
    let app = build_test_app();

    let lines = render_lines(&app, 60, 14);

    assert!(
        lines.iter().any(|line| line.contains("┌ log ")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("(no log)")),
        "lines: {:?}",
        lines
    );
    let footer_row = lines.len() - 2;
    assert!(lines[footer_row].contains("DAW"), "lines: {:?}", lines);
}

#[test]
fn draw_shows_recent_log_lines() {
    let app = build_test_app();
    {
        let mut log_lines = app.log_lines.lock().unwrap();
        log_lines.push_back("old".to_string());
        log_lines.push_back("meas1: cache hit".to_string());
        log_lines.push_back("meas2: render".to_string());
        log_lines.push_back("meas3: empty -> silence".to_string());
    }

    let lines = render_lines(&app, 60, 14);

    assert!(
        !lines.iter().any(|line| line.contains("old")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines.iter().any(|line| line.contains("meas2: render")),
        "lines: {:?}",
        lines
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("meas3: empty -> silence")),
        "lines: {:?}",
        lines
    );
    assert!(
        !lines.iter().any(|line| line.contains("meas1: cache hit")),
        "lines: {:?}",
        lines
    );
}

#[test]
fn draw_highlights_future_append_in_monokai_pink() {
    let app = build_test_app();
    {
        let mut log_lines = app.log_lines.lock().unwrap();
        log_lines.push_back("play: queue meas2 append lead=48ms (target_margin=50ms)".to_string());
    }

    let buffer = render_buffer(&app, 80, 12);
    let (x, y) = find_text_ignoring_spaces(&buffer, "play:queuemeas2appendlead=48ms");

    assert_eq!(
        buffer.cell((x, y)).unwrap().fg,
        MONOKAI_PINK,
        "future append log should use Monokai pink"
    );
}

#[test]
fn draw_highlights_failed_logs_in_red() {
    let app = build_test_app();
    {
        let mut log_lines = app.log_lines.lock().unwrap();
        log_lines.push_back("play: audio init failed".to_string());
    }

    let buffer = render_buffer(&app, 80, 12);
    let (x, y) = find_text_ignoring_spaces(&buffer, "play:audioinitfailed");

    assert_eq!(
        buffer.cell((x, y)).unwrap().fg,
        Color::Red,
        "failed logs should use error red"
    );
}
