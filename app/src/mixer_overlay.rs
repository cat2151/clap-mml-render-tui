use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui_theme::{
    cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW,
};

// mixer の音量ドメイン（定数・dB 調整・ゲイン変換）は画面横断で共有するため
// `cmrt-tui-core` へ切り出した。従来の `crate::mixer_overlay::*` パスは再エクスポートで維持する。
pub(crate) use cmrt_tui_core::mixer::{
    adjust_volume_db, volume_db_to_gain, MIXER_MAX_DB, MIXER_MIN_DB, MIXER_STEP_DB,
};

const TRACK_COLUMN_WIDTH: u16 = 8;
const TRACK_HEADER_WIDTH: usize = TRACK_COLUMN_WIDTH as usize;

pub(crate) struct MixerOverlayTrack {
    pub(crate) label: String,
    pub(crate) volume_db: i32,
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

fn visible_track_range(
    track_count: usize,
    selected_track: usize,
    inner: Rect,
) -> std::ops::Range<usize> {
    let visible_tracks =
        usize::from((inner.width.saturating_sub(7) / TRACK_COLUMN_WIDTH).max(1)).min(track_count);
    let selected_track = selected_track.min(track_count.saturating_sub(1));
    let max_start = track_count.saturating_sub(visible_tracks);
    let start = selected_track
        .saturating_sub(visible_tracks.saturating_sub(1))
        .min(max_start);
    start..(start + visible_tracks).min(track_count)
}

pub(crate) fn draw_mixer_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    tracks: &[MixerOverlayTrack],
    selected_track: usize,
) {
    let popup = crate::ui_utils::centered_rect(92, 76, area);
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
    let track_range = visible_track_range(tracks.len(), selected_track, inner);
    if track_range.is_empty() {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let mut header_spans = vec![Span::styled("       ", Style::default().fg(MONOKAI_GRAY))];
    for track in track_range.clone() {
        let is_selected = track == selected_track;
        let style = if is_selected {
            cursor_highlight_style(Style::default().fg(MONOKAI_FG))
        } else {
            Style::default().fg(MONOKAI_YELLOW)
        };
        header_spans.push(Span::styled(
            format!("{:^TRACK_HEADER_WIDTH$}", tracks[track].label),
            style,
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(header_spans)), chunks[0]);

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
            let meter = if is_active { "[##]    " } else { "[  ]    " };
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

    let mut value_spans = vec![Span::styled("       ", Style::default().fg(MONOKAI_GRAY))];
    for track in track_range.clone() {
        let style = if track == selected_track {
            cursor_highlight_style(Style::default().fg(MONOKAI_FG))
        } else {
            Style::default().fg(MONOKAI_FG)
        };
        value_spans.push(Span::styled(
            format!(
                "{:^TRACK_HEADER_WIDTH$}",
                format!("{:+}dB", tracks[track].volume_db)
            ),
            style,
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(value_spans)), chunks[2]);

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
