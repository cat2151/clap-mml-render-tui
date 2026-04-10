use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{
    super::{CacheState, DawApp, DawMode},
    cache_indicator, cache_indicator_color, cache_text_color, ANIM_FRAME_COUNT, ANIM_FRAME_MS,
    MONOKAI_BG, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PURPLE, MONOKAI_YELLOW,
};
use crate::ui_theme::cursor_highlight_style;

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

    let solo_mode_active = app.solo_mode_active();
    let ab_repeat_markers = app.ab_repeat_state().marker_indices();

    // ヘッダ行（列ラベル）
    let mut header_spans = vec![Span::styled("     ", Style::default())];
    for m in 0..=app.measures {
        let (label, style) = if m == 0 {
            (" Init".to_string(), Style::default().fg(MONOKAI_GRAY))
        } else {
            let measure_index = m - 1;
            match ab_repeat_markers {
                Some((start_measure_index, end_measure_index))
                    if start_measure_index == measure_index
                        && end_measure_index == measure_index =>
                {
                    (
                        format!("AB{m:<3}"),
                        Style::default()
                            .fg(MONOKAI_YELLOW)
                            .add_modifier(Modifier::BOLD),
                    )
                }
                Some((start_measure_index, _)) if start_measure_index == measure_index => (
                    format!("A{m:<4}"),
                    Style::default()
                        .fg(MONOKAI_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Some((_, end_measure_index)) if end_measure_index == measure_index => (
                    format!("B{m:<4}"),
                    Style::default()
                        .fg(MONOKAI_PURPLE)
                        .add_modifier(Modifier::BOLD),
                ),
                _ => (format!("M{m:<4}"), Style::default().fg(MONOKAI_GRAY)),
            }
        };
        header_spans.push(Span::styled(label, style));
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
    for (t, (data_row, cache_row)) in app
        .data
        .iter()
        .zip(cache_states.iter())
        .enumerate()
        .take(app.tracks)
    {
        let row_y = area.y + 1 + (t as u16) * 2;
        if row_y + 1 >= area.y + area.height {
            break;
        }

        let is_cursor_track = t == app.cursor_track;
        let is_muted_track = solo_mode_active && !app.track_is_audible(t);
        let label_fg = MONOKAI_GRAY;

        // 行 1: track ラベル + セル内容 (4 chars each)
        let track_label = if t == 0 {
            "Tempo".to_string()
        } else {
            format!("T{:<2}  ", t)
        };
        let label_style = if is_cursor_track {
            cursor_highlight_style(Style::default().fg(label_fg))
        } else {
            Style::default().fg(label_fg)
        };
        let mut row1: Vec<Span> = vec![Span::styled(track_label, label_style)];

        // INSERTモード時はカーソルtrackのインジケータ行（行2）が不要なので生成をスキップする。
        let show_indicators = !(is_cursor_track && app.mode == DawMode::Insert);
        let mut row2: Vec<Span> = if show_indicators {
            vec![Span::styled("     ", Style::default())]
        } else {
            vec![]
        };

        for (m, (mml, cs)) in data_row
            .iter()
            .zip(cache_row.iter())
            .enumerate()
            .take(app.measures + 1)
        {
            let is_cursor = is_cursor_track && m == app.cursor_measure;

            // セル表示 (4 chars)
            let display: String = if mml.is_empty() {
                "    ".to_string()
            } else {
                let s: String = mml.chars().take(4).collect();
                format!("{:<4}", s)
            };

            let fg = if is_muted_track {
                MONOKAI_GRAY
            } else {
                cache_text_color(cs)
            };
            let style = if is_cursor {
                cursor_highlight_style(Style::default().fg(fg))
            } else {
                Style::default().fg(fg).bg(MONOKAI_BG)
            };

            row1.push(Span::styled(format!("{} ", display), style));

            // 状態インジケータ (4 chars + 1 space): INSERTモードのカーソルtrackはスキップ
            if show_indicators {
                let indicator = if solo_mode_active && m == 0 && t > 0 {
                    if app.track_is_soloed(t) {
                        "solo "
                    } else {
                        "mute "
                    }
                } else {
                    cache_indicator(cs, anim_frame)
                };
                let ind_fg = if solo_mode_active && m == 0 && t > 0 {
                    if app.track_is_soloed(t) {
                        MONOKAI_FG
                    } else {
                        MONOKAI_GRAY
                    }
                } else if is_muted_track {
                    MONOKAI_GRAY
                } else {
                    cache_indicator_color(cs)
                };
                let style = if is_cursor {
                    cursor_highlight_style(Style::default().fg(ind_fg))
                } else {
                    Style::default().fg(ind_fg)
                };
                row2.push(Span::styled(indicator, style));
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
