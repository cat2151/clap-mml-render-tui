use super::*;

#[test]
fn draw_shows_mml_and_uncached_dot_before_cache_is_ready() {
    let mut app = build_test_app();
    app.data[1][1] = "cdef".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Pending;
    }

    let lines = render_lines(&app, 40, 14);

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
fn draw_renders_pending_indicator_in_visible_color() {
    let app = build_test_app();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Pending;
    }

    let buffer = render_buffer(&app, 40, 14);

    assert_eq!(buffer.cell((11, 5)).unwrap().symbol(), ".");
    assert_eq!(buffer.cell((11, 5)).unwrap().fg, MONOKAI_FG);
}

#[test]
fn draw_uses_contrast_background_for_selected_grid_cell_without_blink() {
    let mut app = build_test_app();
    app.data[0][0] = "t120".to_string();

    let buffer = render_buffer(&app, 40, 14);
    let (x, y) = find_text_ignoring_spaces(&buffer, "t120");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_GRAY);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_GRAY));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}

#[test]
fn draw_places_playback_status_and_loop_summary_above_footer() {
    let app = build_test_app();
    {
        let mut play_state = app.play_state.lock().unwrap();
        *play_state = DawPlayState::Playing;
    }
    {
        let mut play_position = app.play_position.lock().unwrap();
        *play_position = Some(PlayPosition {
            measure_index: 1,
            measure_start: std::time::Instant::now(),
        });
    }
    {
        let mut play_measure_mmls = app.play_measure_mmls.lock().unwrap();
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
        let mut ab_repeat = app.ab_repeat.lock().unwrap();
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
fn draw_keeps_footer_on_last_row_when_idle() {
    let app = build_test_app();

    let lines = render_lines(&app, 120, 10);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    let play_row = lines.len() - 5;
    let info_row = lines.len() - 4;
    let render_row = lines.len() - 3;
    let footer_row = lines.len() - 2;

    assert!(!lines[play_row].contains('▶'), "lines: {:?}", lines);
    assert!(
        !lines[info_row].contains("loop meas :"),
        "lines: {:?}",
        lines
    );
    assert!(
        normalized_lines[render_row].contains("並列render中:0"),
        "lines: {:?}",
        lines
    );
    assert!(lines[footer_row].contains("DAW"), "lines: {:?}", lines);
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

#[test]
fn draw_keeps_footer_color_cyan_across_play_states() {
    for play_state in [
        DawPlayState::Idle,
        DawPlayState::Playing,
        DawPlayState::Preview,
    ] {
        let app = build_test_app();
        {
            let mut state = app.play_state.lock().unwrap();
            *state = play_state;
        }

        let buffer = render_buffer(&app, 120, 10);

        assert_eq!(
            buffer.cell((1, 8)).unwrap().fg,
            MONOKAI_CYAN,
            "footer color should stay cyan"
        );
    }
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

    assert_eq!(
        buffer.cell((2, 6)).unwrap().fg,
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

    assert_eq!(
        buffer.cell((2, 6)).unwrap().fg,
        Color::Red,
        "failed logs should use error red"
    );
}
