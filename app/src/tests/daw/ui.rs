use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};
use tui_textarea::TextArea;

use crate::config::Config;

use super::{
    super::{CacheState, CellCache, DawApp, DawMode, DawPlayState, MEASURES},
    cache_indicator, cache_indicator_color, cache_text_color, draw, loop_measure_summary_label,
    loop_status_label, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_PINK,
};

fn build_test_app() -> DawApp {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        data: vec![vec![String::new(); measures + 1]; tracks],
        cursor_track: 0,
        cursor_measure: 0,
        mode: DawMode::Normal,
        textarea: TextArea::default(),
        cfg: Arc::new(Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: None,
            patches_dir: None,
            daw_tracks: tracks,
            daw_measures: measures,
        }),
        entry_ptr: 0,
        tracks,
        measures,
        cache: Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); measures + 1];
            tracks
        ])),
        cache_tx,
        render_lock: Arc::new(Mutex::new(())),
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        play_position: Arc::new(Mutex::new(None)),
        play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
        play_measure_track_mmls: Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures])),
        play_measure_samples: Arc::new(Mutex::new(0)),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        mixer_cursor_track: 1,
        play_track_gains: Arc::new(Mutex::new(vec![0.0; tracks])),
    }
}

fn render_lines(app: &DawApp, width: u16, height: u16) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn render_buffer(app: &DawApp, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn loop_status_label_single_measure_shows_single_measure_loop() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();

    assert_eq!(
        loop_status_label(&mmls),
        Some("loop: meas1のみ (1小節)".to_string())
    );
}

#[test]
fn loop_status_label_uses_last_non_empty_measure() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();
    mmls[2] = "g".to_string();

    assert_eq!(
        loop_status_label(&mmls),
        Some("loop: meas1〜meas3 (3小節)".to_string())
    );
}

#[test]
fn loop_status_label_all_empty_returns_none() {
    assert_eq!(loop_status_label(&vec![String::new(); MEASURES]), None);
}

#[test]
fn loop_measure_summary_label_lists_loop_and_empty_ranges() {
    let mut mmls = vec![String::new(); MEASURES];
    mmls[0] = "c".to_string();

    assert_eq!(
        loop_measure_summary_label(&mmls),
        Some("loop meas : meas 1, empty meas : meas 2～8".to_string())
    );
}

#[test]
fn cache_indicator_uses_single_dot_for_uncached_cells() {
    assert_eq!(cache_indicator(&CacheState::Pending, 0), ".    ");
    assert_eq!(cache_indicator(&CacheState::Pending, 2), ".    ");
}

#[test]
fn cache_indicator_animates_only_while_rendering() {
    assert_eq!(cache_indicator(&CacheState::Rendering, 0), ".    ");
    assert_eq!(cache_indicator(&CacheState::Rendering, 1), "..   ");
    assert_eq!(cache_indicator(&CacheState::Rendering, 2), "...  ");
}

#[test]
fn cache_text_color_keeps_uncached_mml_visible() {
    assert_eq!(cache_text_color(&CacheState::Pending), MONOKAI_FG);
    assert_eq!(cache_text_color(&CacheState::Rendering), MONOKAI_FG);
}

#[test]
fn cache_indicator_color_keeps_pending_animation_visible() {
    assert_eq!(cache_indicator_color(&CacheState::Empty), MONOKAI_GRAY);
    assert_eq!(cache_indicator_color(&CacheState::Pending), MONOKAI_FG);
    assert_eq!(cache_indicator_color(&CacheState::Rendering), MONOKAI_FG);
    assert_eq!(cache_indicator_color(&CacheState::Ready), MONOKAI_GRAY);
    assert_eq!(cache_indicator_color(&CacheState::Error), Color::Red);
}

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
        *play_position = Some(super::super::PlayPosition {
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
