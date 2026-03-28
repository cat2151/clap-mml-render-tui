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

    let play_row = lines.len() - 4;
    let info_row = lines.len() - 3;
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

    let normalized_lines: Vec<String> = render_lines(&app, 80, 12)
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

    let play_row = lines.len() - 4;
    let info_row = lines.len() - 3;
    let footer_row = lines.len() - 2;

    assert!(!lines[play_row].contains('▶'), "lines: {:?}", lines);
    assert!(
        !lines[info_row].contains("loop meas :"),
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
    app.data[2][1] = "gabc".to_string();
    app.solo_tracks[1] = true;

    let buffer = render_buffer(&app, 60, 20);

    assert_eq!(buffer.cell((1, 6)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((11, 6)).unwrap().fg, MONOKAI_GRAY);
}

#[test]
fn help_does_not_show_old_semicolon_guidance() {
    let mut app = build_test_app();
    app.mode = DawMode::Help;

    // ratatui のテスト描画では全角文字の間に空白が入るため、空白を除去して比較する。
    let normalized_lines: Vec<String> = render_lines(&app, 100, 30)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("ヘルプ(Keybinds)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("K/?:ヘルプ(このページ)")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("Ctrl+C/X/V:コピー/カット/ペースト")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("s:solotoggle")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("m:mixeroverlay")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        normalized_lines
            .iter()
            .any(|line| line.contains("a:off→start固定/end追従→end固定→off")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("Ctrl+C:強制終了")),
        "lines: {:?}",
        normalized_lines
    );
    assert!(
        !normalized_lines
            .iter()
            .any(|line| line.contains("分割して下のtrackに追加")),
        "lines: {:?}",
        normalized_lines
    );
}

#[test]
fn draw_shows_mixer_overlay_with_track_labels_and_db_values() {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.mixer_cursor_track = 1;
    app.track_volumes_db[1] = -3;
    app.track_volumes_db[2] = 6;

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
fn draw_highlights_selected_mixer_track_in_cyan() {
    let mut app = build_test_app();
    app.mode = DawMode::Mixer;
    app.mixer_cursor_track = 1;

    let buffer = render_buffer(&app, 100, 30);
    let cyan_positions: Vec<(u16, u16)> = (0..100)
        .flat_map(|x| (0..30).map(move |y| (x, y)))
        .filter(|(x, y)| {
            let cell = buffer.cell((*x, *y)).unwrap();
            cell.symbol() == "t" && cell.fg == MONOKAI_CYAN
        })
        .collect();

    assert!(
        !cyan_positions.is_empty(),
        "selected mixer track label should be cyan"
    );
}
