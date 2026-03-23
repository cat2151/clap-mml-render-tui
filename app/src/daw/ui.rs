//! DAW モードの描画

mod grid;
mod help;
mod status;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Color,
    Frame,
};

use super::{CacheState, DawApp, DawMode};

/// Pending インジケータのアニメーション 1 フレームの長さ（ミリ秒）
const ANIM_FRAME_MS: u128 = 250;
/// Pending インジケータのアニメーションフレーム数（"." / ".." / "..."）
const ANIM_FRAME_COUNT: u128 = 3;

fn cache_text_color(cs: &CacheState) -> Color {
    match cs {
        CacheState::Empty => Color::DarkGray,
        CacheState::Pending | CacheState::Rendering | CacheState::Ready => Color::White,
        CacheState::Error => Color::Red,
    }
}

fn cache_indicator(cs: &CacheState, anim_frame: u128) -> &'static str {
    match cs {
        CacheState::Empty => "     ",
        CacheState::Pending => ".    ",
        CacheState::Rendering => match anim_frame {
            0 => ".    ",
            1 => "..   ",
            _ => "...  ",
        },
        CacheState::Ready => "     ",
        CacheState::Error => "✗    ",
    }
}

fn loop_status_label(mmls: &[String]) -> Option<String> {
    super::playback::effective_measure_count(mmls).map(|count| {
        if count == 1 {
            "loop: meas1のみ (1小節)".to_string()
        } else {
            format!("loop: meas1〜meas{count} ({count}小節)")
        }
    })
}

pub(super) fn draw(app: &DawApp, f: &mut Frame) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    grid::draw_grid(app, f, chunks[0]);
    status::draw_status(app, f, chunks[1], chunks[2]);

    if app.mode == DawMode::Help {
        help::draw_help(f, area);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};
    use tui_textarea::TextArea;

    use crate::config::Config;

    use super::{
        super::{CacheState, CellCache, DawApp, DawMode, DawPlayState, MEASURES},
        cache_indicator, cache_text_color, draw, loop_status_label,
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
            play_position: Arc::new(Mutex::new(None)),
            play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
            play_measure_samples: Arc::new(Mutex::new(0)),
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
        assert_eq!(cache_text_color(&CacheState::Pending), Color::White);
        assert_eq!(cache_text_color(&CacheState::Rendering), Color::White);
    }

    #[test]
    fn draw_shows_mml_and_uncached_dot_before_cache_is_ready() {
        let mut app = build_test_app();
        app.data[1][1] = "cdef".to_string();
        {
            let mut cache = app.cache.lock().unwrap();
            cache[1][1].state = CacheState::Pending;
        }

        let lines = render_lines(&app, 40, 8);

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
    fn draw_places_playback_status_on_second_to_last_row_left_edge() {
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

        let lines = render_lines(&app, 120, 8);

        assert!(lines[6].starts_with("▶ meas2, beat"), "lines: {:?}", lines);
        assert!(lines[6].contains("loop:"), "lines: {:?}", lines);
        assert!(lines[6].contains("meas1"), "lines: {:?}", lines);
        assert!(lines[7].starts_with("DAW"), "lines: {:?}", lines);
        assert!(!lines[7].contains("▶"), "lines: {:?}", lines);
    }

    #[test]
    fn draw_keeps_footer_on_last_row_when_idle() {
        let app = build_test_app();

        let lines = render_lines(&app, 120, 8);

        assert_eq!(lines[6], "", "lines: {:?}", lines);
        assert!(lines[7].starts_with("DAW"), "lines: {:?}", lines);
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

            let buffer = render_buffer(&app, 120, 8);

            assert_eq!(
                buffer.cell((0, 7)).unwrap().fg,
                Color::Cyan,
                "footer color should stay cyan"
            );
        }
    }
}
