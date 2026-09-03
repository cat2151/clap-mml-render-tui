//! mixer オーバーレイの描画ウィジェット（DAW / loop browser 共通）。

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::mixer::{MIXER_MAX_DB, MIXER_MIN_DB, MIXER_STEP_DB};
use crate::theme::{
    cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW,
};
use crate::ui::centered_rect;

/// 行頭の dB 目盛り（`  +6dB `）の桁数。track の列はこのうしろから始まる。
const LEVEL_LABEL_WIDTH: usize = 7;
/// track 1 列の最小桁数。狭い端末ではここまで詰めて、足りなければ横スクロールする。
const MIN_TRACK_COLUMN_WIDTH: usize = 8;
/// track 1 列の最大桁数。これ以上広げても音色名の余白が増えるだけ。
const MAX_TRACK_COLUMN_WIDTH: usize = 14;
/// ヘッダに音色情報（role / 音色名）を出すときの行数。
const HEADER_LINES_WITH_PATCH: u16 = 3;
/// 音色情報を出さない（loop browser など）ときのヘッダ行数。
const HEADER_LINES_PLAIN: u16 = 1;
/// ヘッダを 3 行にするために必要な inner の高さ（ヘッダ 3 + メーター 1 + 値 1 + ヒント 1）。
const MIN_HEIGHT_FOR_PATCH_HEADER: u16 = 6;

pub struct MixerOverlayTrack {
    pub label: String,
    pub volume_db: i32,
    /// その track の音色の用途（`lead` / `*chord` など）。出せない画面は `None`。
    pub role: Option<String>,
    /// その track の音色名。出せない画面は `None`。
    pub patch: Option<String>,
}

fn mixer_levels_db() -> Vec<i32> {
    let mut levels = Vec::new();
    let mut current = MIXER_MIN_DB;
    while current <= MIXER_MAX_DB {
        levels.push(current);
        current += MIXER_STEP_DB;
    }
    levels.reverse();
    levels
}

/// 全 track が収まる範囲で最大の列幅。
///
/// 音色名は列幅がそのまま読める文字数になるので、広い端末では広く取る。
/// 収まらない端末では最小幅まで詰め、それでも溢れる分は横スクロールへ回す
/// （列を削って詰め込むより、幅を保って送るほうが読める）。
fn track_column_width(inner_width: u16, track_count: usize) -> usize {
    let available = usize::from(inner_width).saturating_sub(LEVEL_LABEL_WIDTH);
    (available / track_count.max(1)).clamp(MIN_TRACK_COLUMN_WIDTH, MAX_TRACK_COLUMN_WIDTH)
}

/// ヘッダの行数。音色情報を持つ track が 1 つも無ければ track 名の 1 行だけ。
fn header_line_count(tracks: &[MixerOverlayTrack], inner_height: u16) -> u16 {
    let has_patch_info = tracks
        .iter()
        .any(|track| track.role.is_some() || track.patch.is_some());
    if has_patch_info && inner_height >= MIN_HEIGHT_FOR_PATCH_HEADER {
        HEADER_LINES_WITH_PATCH
    } else {
        HEADER_LINES_PLAIN
    }
}

fn visible_track_range(
    track_count: usize,
    selected_track: usize,
    inner: Rect,
    column_width: usize,
) -> std::ops::Range<usize> {
    let visible_tracks = (usize::from(inner.width).saturating_sub(LEVEL_LABEL_WIDTH)
        / column_width.max(1))
    .max(1)
    .min(track_count);
    let selected_track = selected_track.min(track_count.saturating_sub(1));
    let max_start = track_count.saturating_sub(visible_tracks);
    let start = selected_track
        .saturating_sub(visible_tracks.saturating_sub(1))
        .min(max_start);
    start..(start + visible_tracks).min(track_count)
}

/// 1 列ぶんのセル。必ず末尾に 1 桁の区切り空白を残して左寄せする。
///
/// 中央寄せにすると、track 名 / role / 音色名の 3 行が列の中でばらばらの位置に出て
/// 縦に読めなくなる。メーター（`[##]`）も左寄せなので、左寄せで揃える。
fn column_cell(text: &str, column_width: usize) -> String {
    let body: String = text.chars().take(column_width.saturating_sub(1)).collect();
    format!("{body:<column_width$}")
}

fn column_style(is_selected: bool, fg: Color) -> Style {
    if is_selected {
        cursor_highlight_style(Style::default().fg(MONOKAI_FG))
    } else {
        Style::default().fg(fg)
    }
}

/// ヘッダ 1 行ぶん。`value` が track ごとの表示文字列を返す。
fn header_line<'a>(
    tracks: &[MixerOverlayTrack],
    track_range: std::ops::Range<usize>,
    selected_track: usize,
    column_width: usize,
    fg: Color,
    value: impl Fn(&MixerOverlayTrack) -> String,
) -> Line<'a> {
    let mut spans = vec![Span::styled(
        " ".repeat(LEVEL_LABEL_WIDTH),
        Style::default().fg(MONOKAI_GRAY),
    )];
    for track in track_range {
        spans.push(Span::styled(
            column_cell(&value(&tracks[track]), column_width),
            column_style(track == selected_track, fg),
        ));
    }
    Line::from(spans)
}

pub fn draw_mixer_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    tracks: &[MixerOverlayTrack],
    selected_track: usize,
) {
    let popup = centered_rect(92, 76, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" mixer ")
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 4 || tracks.is_empty() {
        return;
    }

    let levels = mixer_levels_db();
    let column_width = track_column_width(inner.width, tracks.len());
    let track_range = visible_track_range(tracks.len(), selected_track, inner, column_width);
    if track_range.is_empty() {
        return;
    }

    let header_lines = header_line_count(tracks, inner.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_lines),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let mut header_rows = vec![header_line(
        tracks,
        track_range.clone(),
        selected_track,
        column_width,
        MONOKAI_YELLOW,
        |track| track.label.clone(),
    )];
    if header_lines == HEADER_LINES_WITH_PATCH {
        header_rows.push(header_line(
            tracks,
            track_range.clone(),
            selected_track,
            column_width,
            MONOKAI_FG,
            |track| track.role.clone().unwrap_or_default(),
        ));
        header_rows.push(header_line(
            tracks,
            track_range.clone(),
            selected_track,
            column_width,
            MONOKAI_GRAY,
            |track| track.patch.clone().unwrap_or_default(),
        ));
    }
    frame.render_widget(Paragraph::new(header_rows), chunks[0]);

    let meter_height = chunks[1].height.min(levels.len() as u16) as usize;
    let visible_levels = &levels[..meter_height];
    let mut meter_lines = Vec::with_capacity(visible_levels.len());
    for &level_db in visible_levels {
        let mut spans = vec![Span::styled(
            format!("{level_db:>4}dB "),
            Style::default().fg(MONOKAI_GRAY),
        )];
        for track in track_range.clone() {
            let is_selected = track == selected_track;
            let is_active = tracks[track].volume_db >= level_db;
            let meter = column_cell(if is_active { "[##]" } else { "[  ]" }, column_width);
            let style = if is_selected {
                let base = if is_active {
                    Style::default().fg(MONOKAI_FG)
                } else {
                    Style::default().fg(MONOKAI_GRAY)
                };
                cursor_highlight_style(base)
            } else if is_active {
                Style::default().fg(MONOKAI_FG)
            } else {
                Style::default().fg(MONOKAI_GRAY)
            };
            spans.push(Span::styled(meter, style));
        }
        meter_lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(meter_lines), chunks[1]);

    frame.render_widget(
        Paragraph::new(header_line(
            tracks,
            track_range.clone(),
            selected_track,
            column_width,
            MONOKAI_FG,
            |track| format!("{:+}dB", track.volume_db),
        )),
        chunks[2],
    );

    let range_hint = if track_range.start > 0 || track_range.end < tracks.len() {
        format!(
            "  view: {}-{}",
            tracks[track_range.start].label,
            tracks[track_range.end.saturating_sub(1)].label
        )
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(format!("h/l: track  j/k: -/+3dB  ESC: close{range_hint}"))
            .style(Style::default().fg(MONOKAI_GRAY)),
        chunks[3],
    );
}

#[cfg(test)]
mod tests;
