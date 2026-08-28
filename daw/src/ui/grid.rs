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
use cmrt_tui_core::theme::cursor_highlight_style;

mod init_cell;
mod measure_cell;

#[cfg(test)]
mod tests;

/// init 列（meas 0）のセル桁数。音色名を `role:音色名` の形で出すため広く取る。
pub(super) const INIT_CELL_WIDTH: usize = 13;
/// meas 1 以降のセル桁数。
pub(super) const MEASURE_CELL_WIDTH: usize = 4;
/// 列と列の間に挟む区切り空白の桁数。
const COLUMN_GAP: usize = 1;
/// 行頭の track ラベル（`Tempo` / `T1`）の桁数。
pub(super) const TRACK_LABEL_WIDTH: usize = 5;

/// その列のセル本体（区切り空白を含まない）の桁数。
pub(super) fn cell_width(measure_index: usize) -> usize {
    if measure_index == 0 {
        INIT_CELL_WIDTH
    } else {
        MEASURE_CELL_WIDTH
    }
}

/// その列が占める桁数（セル本体 + 区切り空白）。
pub(super) fn column_width(measure_index: usize) -> usize {
    cell_width(measure_index) + COLUMN_GAP
}

/// grid 領域の左端から数えた、その列の内容が始まる x オフセット。
///
/// 列位置の計算はこの関数に閉じること。ヘッダ・セル・インジケータの 3 行が
/// 縦に揃うかどうかは、すべてこの 1 か所に依存する。
pub(super) fn column_x_offset(measure_index: usize) -> u16 {
    let offset = TRACK_LABEL_WIDTH + (0..measure_index).map(column_width).sum::<usize>();
    offset as u16
}

pub(super) fn draw_grid(app: &DawApp, f: &mut Frame, area: Rect, cache_states: &[Vec<CacheState>]) {
    let solo_mode_active = app.solo_mode_active();
    let ab_repeat_markers = app.ab_repeat_state().marker_indices();
    // init 列の role 表示に使う catalog。1 描画につき 1 回だけ lock を取る。
    let catalog = app.catalog_snapshot();

    // ヘッダ行（列ラベル）
    // 行頭の空白は、init 列が始まる x までを埋める。
    let mut header_spans = vec![Span::styled(
        " ".repeat(column_x_offset(0) as usize),
        Style::default(),
    )];
    for m in 0..=app.editor.measures {
        let (label, style) = if m == 0 {
            ("Init".to_string(), Style::default().fg(MONOKAI_GRAY))
        } else {
            let measure_index = m - 1;
            match ab_repeat_markers {
                Some((start_measure_index, end_measure_index))
                    if start_measure_index == measure_index
                        && end_measure_index == measure_index =>
                {
                    (
                        format!("AB{m}"),
                        Style::default()
                            .fg(MONOKAI_YELLOW)
                            .add_modifier(Modifier::BOLD),
                    )
                }
                Some((start_measure_index, _)) if start_measure_index == measure_index => (
                    format!("A{m}"),
                    Style::default()
                        .fg(MONOKAI_GREEN)
                        .add_modifier(Modifier::BOLD),
                ),
                Some((_, end_measure_index)) if end_measure_index == measure_index => (
                    format!("B{m}"),
                    Style::default()
                        .fg(MONOKAI_PURPLE)
                        .add_modifier(Modifier::BOLD),
                ),
                _ => (format!("M{m}"), Style::default().fg(MONOKAI_GRAY)),
            }
        };
        header_spans.push(Span::styled(
            format!("{label:<width$}", width = column_width(m)),
            style,
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
    for (t, (data_row, cache_row)) in app
        .editor
        .data
        .iter()
        .zip(cache_states.iter())
        .enumerate()
        .take(app.editor.tracks)
    {
        let row_y = area.y + 1 + (t as u16) * 2;
        if row_y + 1 >= area.y + area.height {
            break;
        }

        let is_cursor_track = t == app.editor.cursor_track;
        let is_muted_track = solo_mode_active && !app.track_is_audible(t);
        let label_fg = MONOKAI_GRAY;

        // 行 1: track ラベル + セル内容 (4 chars each)
        let track_label = format!(
            "{:<width$}",
            crate::tracks::track_label(t),
            width = TRACK_LABEL_WIDTH
        );
        let label_style = if is_cursor_track {
            cursor_highlight_style(Style::default().fg(label_fg))
        } else {
            Style::default().fg(label_fg)
        };
        let mut row1: Vec<Span> = vec![Span::styled(track_label, label_style)];

        // INSERTモード時はカーソルtrackのインジケータ行（行2）が不要なので生成をスキップする。
        let show_indicators = !(is_cursor_track && app.mode == DawMode::Insert);
        let mut row2: Vec<Span> = if show_indicators {
            vec![Span::styled(
                " ".repeat(column_x_offset(0) as usize),
                Style::default(),
            )]
        } else {
            vec![]
        };

        for (m, (mml, cs)) in data_row
            .iter()
            .zip(cache_row.iter())
            .enumerate()
            .take(app.editor.measures + 1)
        {
            let is_cursor = is_cursor_track && m == app.editor.cursor_measure;

            // セル表示（列ごとの桁数。init 列だけ広い）
            let width = cell_width(m);
            // 手書きのセルが優先。空でも chord 行から生成されるセルは音が鳴るので、
            // 何が鳴るのかを chord 行から借りて出す（空セル = 無音ではない）。
            let text: Option<String> = if mml.trim().is_empty() {
                measure_cell::generated_cell_text(&app.editor.data, t, m)
            } else if m == 0 {
                // init 列だけ `role:音色名` / `4/4 t120` へ組み直す。
                // 組み直せないセル（生 MML）は従来どおり先頭 `width` 文字。
                init_cell::init_cell_text(t, mml, catalog.as_deref()).or_else(|| Some(mml.clone()))
            } else {
                Some(mml.clone())
            };
            // 紫 = 手書きではなく chord 行に由来する表示。
            let borrowed_from_chord_row = mml.trim().is_empty() && text.is_some();
            let display: String = match &text {
                Some(text) => {
                    let s: String = text.chars().take(width).collect();
                    format!("{s:<width$}")
                }
                None => " ".repeat(width),
            };

            let fg = if is_muted_track {
                MONOKAI_GRAY
            } else if borrowed_from_chord_row
                || (m == 0 && crate::mml::track_generates_from_chord_row(&app.editor.data, t))
            {
                MONOKAI_PURPLE
            } else {
                cache_text_color(cs)
            };
            let style = if is_cursor {
                cursor_highlight_style(Style::default().fg(fg))
            } else {
                Style::default().fg(fg).bg(MONOKAI_BG)
            };

            row1.push(Span::styled(format!("{} ", display), style));

            // 状態インジケータ（列と同じ桁数）: INSERTモードのカーソルtrackはスキップ
            if show_indicators {
                let solo_label = solo_mode_active && m == 0 && t >= crate::FIRST_PLAYABLE_TRACK;
                // init 列のインジケータ行は空いているので、生成対象 track だけ
                // chord2mml への指定を出す（solo 表示のほうが優先）。
                let chord_directive = if solo_label || m != 0 {
                    None
                } else {
                    init_cell::init_indicator_text(t, mml)
                };
                let indicator_text = if solo_label {
                    if app.track_is_soloed(t) {
                        "solo".to_string()
                    } else {
                        "mute".to_string()
                    }
                } else if let Some(directive) = &chord_directive {
                    directive.clone()
                } else {
                    cache_indicator(cs, anim_frame).to_string()
                };
                // 列幅を越える指定はここで切る（隣の列を侵食させない）。
                let indicator: String = indicator_text
                    .trim_end()
                    .chars()
                    .take(column_width(m))
                    .collect();
                let indicator = format!("{indicator:<width$}", width = column_width(m));
                let ind_fg = if solo_label {
                    if app.track_is_soloed(t) {
                        MONOKAI_FG
                    } else {
                        MONOKAI_GRAY
                    }
                } else if is_muted_track {
                    MONOKAI_GRAY
                } else if chord_directive.is_some() {
                    MONOKAI_PURPLE
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
