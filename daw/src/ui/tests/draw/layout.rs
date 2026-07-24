use super::*;

#[test]
fn draw_shows_outer_border_in_monokai_cyan() {
    let app = build_test_app();

    let buffer = render_buffer(&app, 60, 10);

    assert_eq!(buffer.cell((0, 0)).unwrap().symbol(), "┌");
    assert_eq!(buffer.cell((59, 0)).unwrap().symbol(), "┐");
    assert_eq!(buffer.cell((0, 9)).unwrap().symbol(), "└");
    assert_eq!(buffer.cell((59, 9)).unwrap().symbol(), "┘");
    assert_eq!(buffer.cell((0, 0)).unwrap().fg, Color::Rgb(102, 217, 239));
}

#[test]
fn draw_shows_normal_mode_title_in_top_border() {
    let app = build_test_app();

    let lines = render_lines(&app, 60, 10);

    assert!(lines[0].contains("[NORMAL] DAW mode"), "lines: {:?}", lines);
}

#[test]
fn draw_shows_insert_mode_title_in_top_border() {
    let mut app = build_test_app();
    app.mode = DawMode::Insert;

    let lines = render_lines(&app, 60, 10);

    assert!(lines[0].contains("[INSERT] DAW mode"), "lines: {:?}", lines);
}

#[test]
fn draw_places_playback_status_and_loop_summary_above_footer() {
    let app = build_test_app();
    {
        let mut play_state = app.playback.play_state.lock().unwrap();
        *play_state = DawPlayState::Playing;
    }
    {
        let mut play_position = app.playback.position.lock().unwrap();
        *play_position = Some(PlayPosition {
            measure_index: 1,
            measure_start: std::time::Instant::now(),
            measure_duration: std::time::Duration::from_secs(1),
        });
    }
    {
        let mut play_measure_mmls = app.playback.measure_mmls.lock().unwrap();
        play_measure_mmls[0] = "c".to_string();
    }

    let lines = render_lines(&app, 120, 10);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    let play_row = lines.len() - 5;
    let info_row = lines.len() - 4;
    let render_row = lines.len() - 3;
    let footer_row = lines.len() - 2;

    assert!(
        lines[play_row].contains("▶ meas2, beat"),
        "lines: {:?}",
        lines
    );
    assert!(lines[play_row].contains("loop:"), "lines: {:?}", lines);
    assert!(lines[play_row].contains("meas1"), "lines: {:?}", lines);
    assert!(
        lines[info_row].contains("loop meas :"),
        "lines: {:?}",
        lines
    );
    assert!(
        lines[info_row].contains("empty meas :"),
        "lines: {:?}",
        lines
    );
    assert!(
        normalized_lines[render_row].contains("並列render中:0"),
        "lines: {:?}",
        lines
    );
    assert!(lines[footer_row].contains("DAW"), "lines: {:?}", lines);
    assert!(!lines[footer_row].contains("▶"), "lines: {:?}", lines);
}

#[test]
fn draw_shows_ab_repeat_markers_and_footer_shortcut() {
    let app = build_test_app();
    {
        let mut ab_repeat = app.playback.ab_repeat.lock().unwrap();
        *ab_repeat = AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        };
    }

    let normalized_lines: Vec<String> = render_lines(&app, FOOTER_WIDE_TEST_WIDTH, 12)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();
    let footer_row = normalized_lines.len() - 2;

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("InitA1B2")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines[footer_row].contains("a:A-B"),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_shows_active_parallel_render_count_above_footer() {
    let app = build_test_app();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Rendering;
        cache[2][1].state = CacheState::Rendering;
    }

    let lines = render_lines(&app, 120, 10);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let render_row = lines.len() - 3;
    let footer_row = lines.len() - 2;

    assert!(
        normalized_lines[render_row].contains("並列render中:2"),
        "lines: {:?}",
        lines
    );
    assert!(lines[footer_row].contains("DAW"), "lines: {:?}", lines);
}
