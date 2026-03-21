//! DAW モードの描画

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{CacheState, DawApp, DawMode, DawPlayState};

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
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    draw_grid(app, f, chunks[0]);
    draw_status(app, f, chunks[1]);

    if app.mode == DawMode::Help {
        draw_help(f, area);
    }
}

fn draw_grid(app: &DawApp, f: &mut Frame, area: Rect) {
    // キャッシュ状態をスナップショットしてからロックを解放する。
    // これによりキャッシュワーカースレッドとの競合を最小化する。
    let cache_states: Vec<Vec<CacheState>> = {
        let cache = app.cache.lock().unwrap();
        (0..app.tracks)
            .map(|t| {
                (0..=app.measures)
                    .map(|m| cache[t][m].state.clone())
                    .collect()
            })
            .collect()
    };

    // ヘッダ行（列ラベル）
    let mut header_spans = vec![Span::styled("     ", Style::default())];
    for m in 0..=app.measures {
        let label = if m == 0 {
            " Init".to_string()
        } else {
            format!(" M{:<2}", m)
        };
        header_spans.push(Span::styled(
            format!("{:<5}", label),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if area.height > 0 {
        f.render_widget(
            Paragraph::new(Line::from(header_spans)),
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
    }

    // Pending セル用アニメーションフレーム（0..ANIM_FRAME_COUNT を ANIM_FRAME_MS ごとに切り替え）
    let anim_frame = {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        (millis / ANIM_FRAME_MS) % ANIM_FRAME_COUNT
    };

    // track 行（2 行ずつ）
    for t in 0..app.tracks {
        let row_y = area.y + 1 + (t as u16) * 2;
        if row_y + 1 >= area.y + area.height {
            break;
        }

        let is_cursor_track = t == app.cursor_track;

        // 行 1: track ラベル + セル内容 (4 chars each)
        let track_label = if t == 0 {
            "Tempo".to_string()
        } else {
            format!("T{:<2}  ", t)
        };
        let mut row1: Vec<Span> = vec![Span::styled(
            track_label,
            if is_cursor_track {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )];

        // INSERTモード時はカーソルtrackのインジケータ行（行2）が不要なので生成をスキップする。
        let show_indicators = !(is_cursor_track && app.mode == DawMode::Insert);
        let mut row2: Vec<Span> = if show_indicators {
            vec![Span::styled("     ", Style::default())]
        } else {
            vec![]
        };

        for m in 0..=app.measures {
            let is_cursor = is_cursor_track && m == app.cursor_measure;
            let mml = &app.data[t][m];
            let cs = &cache_states[t][m];

            // セル表示 (4 chars)
            let display: String = if mml.is_empty() {
                "    ".to_string()
            } else {
                let s: String = mml.chars().take(4).collect();
                format!("{:<4}", s)
            };

            let (fg, bg) = if is_cursor {
                (Color::Black, Color::Cyan)
            } else {
                (cache_text_color(cs), Color::Reset)
            };

            row1.push(Span::styled(
                format!("{} ", display),
                Style::default().fg(fg).bg(bg),
            ));

            // 状態インジケータ (4 chars + 1 space): INSERTモードのカーソルtrackはスキップ
            if show_indicators {
                let indicator = cache_indicator(cs, anim_frame);
                let ind_fg = if is_cursor {
                    Color::Cyan
                } else {
                    match cs {
                        CacheState::Empty => Color::DarkGray,
                        CacheState::Pending => Color::DarkGray,
                        CacheState::Rendering => Color::DarkGray,
                        CacheState::Ready => Color::DarkGray,
                        CacheState::Error => Color::Red,
                    }
                };
                row2.push(Span::styled(indicator, Style::default().fg(ind_fg)));
            }
        }

        f.render_widget(
            Paragraph::new(Line::from(row1)),
            Rect {
                x: area.x,
                y: row_y,
                width: area.width,
                height: 1,
            },
        );

        // INSERTモード時は、カーソルtrackのインジケータ行にインラインで textarea を描画する。
        if show_indicators {
            f.render_widget(
                Paragraph::new(Line::from(row2)),
                Rect {
                    x: area.x,
                    y: row_y + 1,
                    width: area.width,
                    height: 1,
                },
            );
        } else {
            f.render_widget(
                &app.textarea,
                Rect {
                    x: area.x,
                    y: row_y + 1,
                    width: area.width,
                    height: 1,
                },
            );
        }
    }
}

fn draw_status(app: &DawApp, f: &mut Frame, area: Rect) {
    // play_state と play_position を一度だけロックしてスナップショットを取る。
    let play_state = app.play_state.lock().unwrap().clone();
    let play_position = app.play_position.lock().unwrap().clone();
    let loop_label = if play_state == DawPlayState::Playing {
        let play_measure_mmls = app.play_measure_mmls.lock().unwrap();
        loop_status_label(&play_measure_mmls)
    } else {
        None
    };

    // 拍子・テンポは常に現在の app 状態から取得することで、
    // hot reload 後もビート表示が正確に保たれる。
    let beat_count = app.beat_numerator();
    let beat_duration_secs = 60.0 / app.tempo_bpm();

    let play_str = match &play_state {
        DawPlayState::Idle => "".to_string(),
        DawPlayState::Playing | DawPlayState::Preview => {
            let label = if play_state == DawPlayState::Preview {
                "PREVIEW".to_string()
            } else {
                loop_label.unwrap_or_else(|| "loop".to_string())
            };
            let pos_str = if let Some(pos) = &play_position {
                let elapsed = pos.measure_start.elapsed().as_secs_f64();
                let raw_beat = (elapsed / beat_duration_secs) as u32;
                let current_beat = (raw_beat % beat_count) + 1;
                format!(
                    "  ▶ meas{}, beat{} ({})",
                    pos.measure_index + 1,
                    current_beat,
                    label
                )
            } else {
                format!("  ▶ 演奏中 ({})", label)
            };
            pos_str
        }
    };

    let text = match app.mode {
        DawMode::Normal => format!(
            "DAW  h/l:小節移動  j/k:track移動  i:INSERT  p:play/stop  r:random音色  K:ヘルプ  d/ESC:戻る  q:終了{}",
            play_str
        ),
        DawMode::Insert => format!(
            "ESC:確定→NORMAL  Enter:確定→次小節{}",
            play_str
        ),
        DawMode::Help => format!(
            "HELP  ESC:キャンセル{}",
            play_str
        ),
    };

    let color = match play_state {
        DawPlayState::Idle => Color::Cyan,
        DawPlayState::Playing => Color::Yellow,
        DawPlayState::Preview => Color::Magenta,
    };

    f.render_widget(Paragraph::new(text).style(Style::default().fg(color)), area);
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use ratatui::{backend::TestBackend, style::Color, Terminal};
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
}

fn draw_help(f: &mut Frame, area: Rect) {
    let popup = crate::ui_utils::centered_rect(60, 80, area);
    f.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  h / ←  : 小節移動（左）"),
        Line::from("  l / →  : 小節移動（右）"),
        Line::from("  j / ↓  : track 移動（下）"),
        Line::from("  k / ↑  : track 移動（上）"),
        Line::from("  H      : 先頭 track へ移動"),
        Line::from("  M      : 中央 track へ移動"),
        Line::from("  L      : 末尾 track へ移動"),
        Line::from("  i      : INSERT モード"),
        Line::from("  p      : 演奏 / 停止"),
        Line::from("  r      : random 音色設定"),
        Line::from("  K      : ヘルプ (このページ)"),
        Line::from("  d/ESC  : TUI に戻る"),
        Line::from("  q      : 終了"),
        Line::from("  Ctrl+C : 強制終了"),
        Line::from(""),
        Line::from(Span::styled(
            "INSERT モード",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ESC   : 確定 → NORMAL"),
        Line::from("  Enter : 確定 → 次小節 → INSERT 継続"),
        Line::from("  ;     : 分割して下の track に追加"),
        Line::from(""),
        Line::from(Span::styled(
            "  [ESC] でキャンセル",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ヘルプ (Keybinds) ")
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        popup,
    );
}
