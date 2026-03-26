use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{
    super::{CacheState, DawApp, DawMode},
    cache_indicator, cache_indicator_color, cache_text_color, ANIM_FRAME_COUNT, ANIM_FRAME_MS,
};

pub(super) fn draw_grid(app: &DawApp, f: &mut Frame, area: Rect) {
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
                    cache_indicator_color(cs)
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
