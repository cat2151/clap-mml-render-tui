use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{
    super::{DawApp, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB},
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW,
};
use crate::ui_theme::blinking_cursor_style;

const TRACK_COLUMN_WIDTH: u16 = 8;
const TRACK_HEADER_WIDTH: usize = TRACK_COLUMN_WIDTH as usize;

fn mixer_levels_db() -> Vec<i32> {
    let mut levels = Vec::new();
    let mut current = MIXER_MIN_DB;
    while current <= MIXER_MAX_DB {
        levels.push(current);
        current += 3;
    }
    levels.reverse();
    levels
}

fn visible_track_range(app: &DawApp, inner: Rect) -> std::ops::Range<usize> {
    let playable_tracks = app.tracks.saturating_sub(FIRST_PLAYABLE_TRACK);
    let visible_tracks = (inner.width.saturating_sub(7) / TRACK_COLUMN_WIDTH)
        .max(1)
        .min(playable_tracks as u16) as usize;
    let selected_offset = app
        .mixer_cursor_track
        .saturating_sub(FIRST_PLAYABLE_TRACK)
        .min(playable_tracks.saturating_sub(1));
    let max_start = playable_tracks.saturating_sub(visible_tracks);
    let start_offset = selected_offset
        .saturating_sub(visible_tracks.saturating_sub(1))
        .min(max_start);
    let start = FIRST_PLAYABLE_TRACK + start_offset;
    start..(start + visible_tracks).min(app.tracks)
}

pub(super) fn draw_mixer(f: &mut Frame, app: &DawApp, area: Rect) {
    let popup = crate::ui_utils::centered_rect(92, 76, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" mixer ")
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    if inner.height < 4 || app.tracks <= FIRST_PLAYABLE_TRACK {
        return;
    }

    let levels = mixer_levels_db();
    let track_range = visible_track_range(app, inner);
    let current_track_count = track_range.end.saturating_sub(track_range.start);
    if current_track_count == 0 {
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
        let is_selected = track == app.mixer_cursor_track;
        let style = if is_selected {
            blinking_cursor_style(Style::default().fg(MONOKAI_FG))
        } else {
            Style::default().fg(MONOKAI_YELLOW)
        };
        header_spans.push(Span::styled(
            format!("{:^TRACK_HEADER_WIDTH$}", format!("track{track}")),
            style,
        ));
    }
    f.render_widget(Paragraph::new(Line::from(header_spans)), chunks[0]);

    let meter_height = chunks[1].height.min(levels.len() as u16) as usize;
    let visible_levels = &levels[..meter_height];
    let mut meter_lines = Vec::with_capacity(visible_levels.len());
    for &level_db in visible_levels {
        let mut spans = vec![Span::styled(
            format!("{level_db:>4}dB "),
            Style::default().fg(MONOKAI_GRAY),
        )];
        for track in track_range.clone() {
            let is_selected = track == app.mixer_cursor_track;
            let is_active = app.track_volume_db(track) >= level_db;
            let meter = if is_active { "[##]    " } else { "[  ]    " };
            let style = if is_selected {
                let base = if is_active {
                    Style::default().fg(MONOKAI_FG)
                } else {
                    Style::default().fg(MONOKAI_GRAY)
                };
                blinking_cursor_style(base)
            } else if is_active {
                Style::default().fg(MONOKAI_FG)
            } else {
                Style::default().fg(MONOKAI_GRAY)
            };
            spans.push(Span::styled(meter, style));
        }
        meter_lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(meter_lines), chunks[1]);

    let mut value_spans = vec![Span::styled("       ", Style::default().fg(MONOKAI_GRAY))];
    for track in track_range.clone() {
        let is_selected = track == app.mixer_cursor_track;
        let style = if is_selected {
            blinking_cursor_style(Style::default().fg(MONOKAI_FG))
        } else {
            Style::default().fg(MONOKAI_FG)
        };
        value_spans.push(Span::styled(
            format!(
                "{:^TRACK_HEADER_WIDTH$}",
                format!("{:+}dB", app.track_volume_db(track))
            ),
            style,
        ));
    }
    f.render_widget(Paragraph::new(Line::from(value_spans)), chunks[2]);

    let range_hint = if track_range.start > FIRST_PLAYABLE_TRACK || track_range.end < app.tracks {
        format!(
            "  view: track{}-track{}",
            track_range.start,
            track_range.end.saturating_sub(1)
        )
    } else {
        String::new()
    };
    f.render_widget(
        Paragraph::new(format!("h/l: track  j/k: -/+3dB  ESC: close{range_hint}"))
            .style(Style::default().fg(MONOKAI_GRAY)),
        chunks[3],
    );
}
