use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::ops::Range;

use super::{focus_border_style, waveforms};
use crate::tui::loop_browser::{LoopBrowser, LoopBrowserPane};
use crate::tui::TuiApp;
use crate::ui_theme::{
    cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_YELLOW,
};

use super::super::status::base_style;

mod analysis;
mod used_wavs;
mod viewport;

use analysis::stretch_label;
use viewport::{fit, keep_visible};

const TRACK_LABEL_WIDTH: usize = 9;
const CELL_WIDTH: usize = 16;
const THREE_PANE_MIN_HEIGHT: u16 = 13;

pub(super) fn draw(app: &mut TuiApp<'_>, frame: &mut Frame<'_>, area: Rect) {
    let show_analysis = area.height >= THREE_PANE_MIN_HEIGHT;
    let panes = if show_analysis {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area)
    };
    let focused = app.loop_browser.state.focus == LoopBrowserPane::Tracks;
    let target_bpm = app.loop_browser.state.target_bpm();
    let track_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " [TRACK LIST BPM{} AUTO-STRETCH] ",
            crate::loop_time_stretch::format_bpm(target_bpm.bpm)
        ))
        .border_style(focus_border_style(focused));
    let used_wavs_block = Block::default()
        .borders(Borders::ALL)
        .title(" [USED WAV / ANALYSIS / CATEGORY / STRETCH SOURCE→TARGET / OUTPUT] ")
        .border_style(focus_border_style(focused));
    let playback_beat = crate::tui::loop_browser::playback::position::current_beat(
        &app.loop_browser.state.playback_position,
        std::time::Instant::now(),
    );
    let beats_per_measure = playback_beat.map_or_else(
        || app.loop_browser.state.beats_per_measure(),
        |position| position.beats_per_measure,
    );
    let playback_marker = PlaybackMarker {
        beat: playback_beat,
        beats_per_measure,
    };
    let waveform_block = Block::default()
        .borders(Borders::ALL)
        .title(waveforms::title(playback_beat))
        .border_style(focus_border_style(focused));
    let track_inner = track_block.inner(panes[0]);
    let (used_wavs_inner, waveform_area) = if show_analysis {
        (Some(used_wavs_block.inner(panes[1])), panes[2])
    } else {
        (None, panes[1])
    };
    let waveform_inner = waveform_block.inner(waveform_area);
    frame.render_widget(track_block, panes[0]);
    if show_analysis {
        frame.render_widget(used_wavs_block, panes[1]);
    }
    frame.render_widget(waveform_block, waveform_area);

    let width = used_wavs_inner.map_or(track_inner.width, |inner| {
        track_inner.width.min(inner.width)
    });
    let width = width.min(waveform_inner.width);
    let height = used_wavs_inner.map_or(track_inner.height, |inner| {
        track_inner.height.min(inner.height)
    });
    let height = height.min(waveform_inner.height);
    if width == 0 || height == 0 {
        return;
    }
    let visible_tracks = usize::from(height.saturating_sub(1)).max(1);
    let visible_measures =
        ((usize::from(width).saturating_sub(TRACK_LABEL_WIDTH)) / CELL_WIDTH).max(1);
    let diagnostics = app
        .loop_browser
        .state
        .stretch_diagnostics
        .lock()
        .unwrap()
        .clone();
    let browser = &mut app.loop_browser.state;
    keep_visible(
        browser.track_cursor,
        visible_tracks,
        &mut browser.track_scroll,
    );
    keep_visible(
        browser.measure_cursor,
        visible_measures,
        &mut browser.measure_scroll,
    );
    let rendered = render_visible_grid(
        browser,
        &diagnostics,
        visible_tracks,
        visible_measures,
        focused,
        target_bpm.bpm,
        playback_marker,
    );

    draw_grid(frame, track_inner, &rendered, GridContent::Wav);
    if let Some(used_wavs_inner) = used_wavs_inner {
        used_wavs::draw(
            frame,
            used_wavs_inner,
            browser,
            &diagnostics,
            target_bpm.bpm,
            focused,
        );
    }
    draw_grid(frame, waveform_inner, &rendered, GridContent::Waveform);
}

#[derive(Clone, Copy)]
enum GridContent {
    Wav,
    Waveform,
}

struct RenderedGrid {
    measures: Range<usize>,
    rows: Vec<RenderedRow>,
    focused: bool,
    track_cursor: usize,
    measure_cursor: usize,
    playback_marker: PlaybackMarker,
}

#[derive(Clone, Copy)]
struct PlaybackMarker {
    beat: Option<crate::tui::loop_browser::playback::position::PlaybackBeat>,
    beats_per_measure: usize,
}

struct RenderedRow {
    track: usize,
    solo: bool,
    muted: bool,
    cells: Vec<RenderedCell>,
}

struct RenderedCell {
    wav_label: String,
    waveform_spans: Vec<Span<'static>>,
    style: Style,
}

fn draw_grid(frame: &mut Frame<'_>, area: Rect, rendered: &RenderedGrid, content: GridContent) {
    let mut lines = Vec::with_capacity(rendered.rows.len() + 1);
    let mut header = vec![Span::styled(
        fit("track", TRACK_LABEL_WIDTH),
        base_style().fg(MONOKAI_CYAN),
    )];
    for measure in rendered.measures.clone() {
        let style = if rendered.focused && measure == rendered.measure_cursor {
            base_style().fg(MONOKAI_YELLOW)
        } else {
            base_style().fg(MONOKAI_CYAN)
        };
        if matches!(content, GridContent::Waveform) {
            waveforms::append_header_cell(
                &mut header,
                measure,
                rendered.playback_marker.beats_per_measure,
                rendered.playback_marker.beat,
                CELL_WIDTH,
            );
        } else {
            header.push(Span::styled(
                fit(&format!("measure {}", measure + 1), CELL_WIDTH),
                style,
            ));
        }
    }
    lines.push(Line::from(header));

    for row in &rendered.rows {
        let (track_label, track_style) = if row.solo {
            (
                format!("solo T{}", row.track + 1),
                base_style().fg(MONOKAI_GREEN),
            )
        } else if row.muted {
            (
                format!("mute T{}", row.track + 1),
                base_style().fg(MONOKAI_GRAY),
            )
        } else {
            (format!("T{}", row.track + 1), base_style().fg(MONOKAI_CYAN))
        };
        let mut spans = vec![Span::styled(
            fit(&track_label, TRACK_LABEL_WIDTH),
            track_style,
        )];
        for (cell, measure) in row.cells.iter().zip(rendered.measures.clone()) {
            let cursor = rendered.focused
                && row.track == rendered.track_cursor
                && measure == rendered.measure_cursor;
            if matches!(content, GridContent::Waveform) {
                waveforms::append_cell(
                    &mut spans,
                    &cell.waveform_spans,
                    row.muted,
                    cursor,
                    rendered.playback_marker.beat,
                    measure,
                    CELL_WIDTH,
                );
                continue;
            }
            let label = match content {
                GridContent::Wav => &cell.wav_label,
                GridContent::Waveform => unreachable!(),
            };
            let cell_style = if row.muted {
                base_style().fg(MONOKAI_GRAY)
            } else {
                cell.style
            };
            let style = if cursor {
                cursor_highlight_style(cell_style)
            } else {
                cell_style
            };
            spans.push(Span::styled(fit(label, CELL_WIDTH), style));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).style(base_style()), area);
}

fn render_visible_grid(
    browser: &LoopBrowser,
    diagnostics: &crate::tui::loop_browser::playback::diagnostics::LoopStretchDiagnostics,
    visible_tracks: usize,
    visible_measures: usize,
    focused: bool,
    target_bpm: f64,
    playback_marker: PlaybackMarker,
) -> RenderedGrid {
    let measures = browser.measure_scroll
        ..(browser.measure_scroll + visible_measures).min(browser.displayed_measure_count());
    let rows = (browser.track_scroll
        ..(browser.track_scroll + visible_tracks).min(browser.track_grid().len()))
        .map(|track| {
            let solo = browser.track_is_soloed(track);
            let muted = !browser.track_is_audible(track);
            let cells = measures
                .clone()
                .map(|measure| render_cell(browser, diagnostics, track, measure, target_bpm))
                .collect();
            RenderedRow {
                track,
                solo,
                muted,
                cells,
            }
        })
        .collect();
    RenderedGrid {
        measures,
        rows,
        focused,
        track_cursor: browser.track_cursor,
        measure_cursor: browser.measure_cursor,
        playback_marker,
    }
}

fn render_cell(
    browser: &LoopBrowser,
    diagnostics: &crate::tui::loop_browser::playback::diagnostics::LoopStretchDiagnostics,
    track: usize,
    measure: usize,
    target_bpm: f64,
) -> RenderedCell {
    let occupied = browser.clip_at(track, measure);
    let repeated = occupied.is_some_and(|(_, clip)| clip.is_previous());
    let clip = occupied.map(|(_, clip)| clip);
    let waveform_spans = waveforms::render_cell(browser, track, measure, CELL_WIDTH);
    let wav_label = occupied
        .map(|(start, clip)| {
            if start == measure {
                if clip.is_previous() {
                    "↻ prev meas".to_string()
                } else {
                    browser.cell_label(&clip.wav)
                }
            } else {
                format!("↳ {}/{}", measure - start + 1, clip.span_measures)
            }
        })
        .unwrap_or_else(|| "·".to_string());
    let Some(clip) = clip else {
        return RenderedCell {
            wav_label,
            waveform_spans,
            style: base_style(),
        };
    };
    let playback_clip = browser.playback_clip(clip);
    let stretch_status = diagnostics.status_for(&playback_clip, target_bpm);
    let exceeds_limits = playback_clip.source_bpm().is_some_and(|source_bpm| {
        crate::loop_time_stretch::exceeds_time_ratio_limits(source_bpm, target_bpm)
    });
    RenderedCell {
        wav_label,
        waveform_spans,
        style: cell_style(stretch_status.as_ref(), repeated, exceeds_limits),
    }
}

fn cell_style(
    stretch_status: Option<&crate::tui::loop_browser::playback::diagnostics::StretchStatusView>,
    repeated: bool,
    exceeds_limits: bool,
) -> Style {
    if stretch_status.is_some_and(|view| {
        matches!(
            view.status,
            crate::tui::loop_browser::playback::diagnostics::StretchStatus::Error(_)
        )
    }) || exceeds_limits
    {
        base_style().fg(Color::Red)
    } else if stretch_status.is_some_and(|view| {
        matches!(
            view.status,
            crate::tui::loop_browser::playback::diagnostics::StretchStatus::Pending
        )
    }) {
        base_style().fg(MONOKAI_CYAN)
    } else if stretch_status.is_some_and(|view| {
        matches!(
            view.status,
            crate::tui::loop_browser::playback::diagnostics::StretchStatus::Ready { info, .. }
                if (info.time_ratio - 1.0).abs() >= f64::EPSILON
        )
    }) {
        base_style().fg(MONOKAI_GREEN)
    } else if repeated
        || stretch_status.is_some_and(|view| {
            matches!(
                view.status,
                crate::tui::loop_browser::playback::diagnostics::StretchStatus::Ready { info, .. }
                    if (info.time_ratio - 1.0).abs() < f64::EPSILON
            )
        })
    {
        base_style().fg(MONOKAI_GRAY)
    } else {
        base_style()
    }
}

#[cfg(test)]
#[path = "tracks/tests.rs"]
mod tests;
