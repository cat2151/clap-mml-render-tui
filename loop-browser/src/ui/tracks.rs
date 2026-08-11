use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::ops::Range;

use super::{focus_border_style, waveforms};
use crate::{LoopBrowser, LoopBrowserPane};
use cmrt_tui_core::theme::{
    cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_YELLOW,
};

use cmrt_tui_core::status::base_style;

mod analysis;
mod used_wavs;
mod viewport;

use analysis::stretch_label;
use viewport::{fit, keep_visible, measure_cell_width, measure_label};

const TRACK_LABEL_WIDTH: usize = 9;
const THREE_PANE_MIN_HEIGHT: u16 = 13;

pub fn draw(state: &mut LoopBrowser, frame: &mut Frame<'_>, area: Rect) {
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
    let focused = state.focus == LoopBrowserPane::Tracks;
    let target_bpm = state.target_bpm();
    let track_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " [TRACK LIST BPM{} {}-STRETCH] ",
            target_bpm.bpm,
            state.bpm_mode().label()
        ))
        .border_style(focus_border_style(focused));
    let used_wavs_block = Block::default()
        .borders(Borders::ALL)
        .title(" [USED WAV / ANALYSIS / CATEGORY / STRETCH SOURCE→TARGET / OUTPUT] ")
        .border_style(focus_border_style(focused));
    let playback_beat = crate::playback::position::current_beat(
        &state.playback_position,
        std::time::Instant::now(),
    );
    let beats_per_measure = playback_beat.map_or_else(
        || state.beats_per_measure(),
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
    let available = usize::from(width).saturating_sub(TRACK_LABEL_WIDTH);
    let cell_width = measure_cell_width(available, state.displayed_measure_count());
    let viewport = GridViewport {
        visible_tracks: usize::from(height.saturating_sub(1)).max(1),
        visible_measures: (available / cell_width).max(1),
        cell_width,
    };
    let diagnostics = state.stretch_diagnostics.lock().unwrap().clone();
    let browser = &mut *state;
    keep_visible(
        browser.track_cursor,
        viewport.visible_tracks,
        &mut browser.track_scroll,
    );
    keep_visible(
        browser.measure_cursor,
        viewport.visible_measures,
        &mut browser.measure_scroll,
    );
    let rendered = render_visible_grid(
        browser,
        &diagnostics,
        viewport,
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

/// 1 回の描画で決まるグリッドの寸法。`cell_width` は小節数に合わせて毎回引き直す。
#[derive(Clone, Copy)]
struct GridViewport {
    visible_tracks: usize,
    visible_measures: usize,
    cell_width: usize,
}

struct RenderedGrid {
    measures: Range<usize>,
    rows: Vec<RenderedRow>,
    cell_width: usize,
    focused: bool,
    track_cursor: usize,
    measure_cursor: usize,
    playback_marker: PlaybackMarker,
}

#[derive(Clone, Copy)]
struct PlaybackMarker {
    beat: Option<crate::playback::position::PlaybackBeat>,
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
                rendered.cell_width,
            );
        } else {
            header.push(Span::styled(
                fit(
                    &measure_label(measure, rendered.cell_width),
                    rendered.cell_width,
                ),
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
                    rendered.cell_width,
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
            spans.push(Span::styled(fit(label, rendered.cell_width), style));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).style(base_style()), area);
}

fn render_visible_grid(
    browser: &LoopBrowser,
    diagnostics: &crate::playback::diagnostics::LoopStretchDiagnostics,
    viewport: GridViewport,
    focused: bool,
    target_bpm: f64,
    playback_marker: PlaybackMarker,
) -> RenderedGrid {
    let measures = browser.measure_scroll
        ..(browser.measure_scroll + viewport.visible_measures)
            .min(browser.displayed_measure_count());
    let measure_seconds = browser.measure_seconds();
    let rows = (browser.track_scroll
        ..(browser.track_scroll + viewport.visible_tracks).min(browser.track_grid().len()))
        .map(|track| {
            let solo = browser.track_is_soloed(track);
            let muted = !browser.track_is_audible(track);
            let cells = measures
                .clone()
                .map(|measure| {
                    render_cell(
                        browser,
                        diagnostics,
                        track,
                        measure,
                        viewport.cell_width,
                        target_bpm,
                        measure_seconds,
                    )
                })
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
        cell_width: viewport.cell_width,
        focused,
        track_cursor: browser.track_cursor,
        measure_cursor: browser.measure_cursor,
        playback_marker,
    }
}

fn render_cell(
    browser: &LoopBrowser,
    diagnostics: &crate::playback::diagnostics::LoopStretchDiagnostics,
    track: usize,
    measure: usize,
    cell_width: usize,
    target_bpm: f64,
    measure_seconds: f64,
) -> RenderedCell {
    let occupied = browser.clip_at(track, measure);
    let repeated = occupied.is_some_and(|(_, clip)| clip.is_previous());
    let clip = occupied.map(|(_, clip)| clip);
    let waveform_spans = waveforms::render_cell(
        browser,
        track,
        measure,
        cell_width,
        measure_seconds,
        target_bpm,
    );
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
        cmrt_loop_browser_domain::time_stretch::exceeds_time_ratio_limits(source_bpm, target_bpm)
    });
    RenderedCell {
        wav_label,
        waveform_spans,
        style: cell_style(stretch_status.as_ref(), repeated, exceeds_limits),
    }
}

fn cell_style(
    stretch_status: Option<&crate::playback::diagnostics::StretchStatusView>,
    repeated: bool,
    exceeds_limits: bool,
) -> Style {
    if stretch_status.is_some_and(|view| {
        matches!(
            view.status,
            crate::playback::diagnostics::StretchStatus::Error(_)
        )
    }) || exceeds_limits
    {
        base_style().fg(Color::Red)
    } else if stretch_status.is_some_and(|view| {
        matches!(
            view.status,
            crate::playback::diagnostics::StretchStatus::Pending
        )
    }) {
        base_style().fg(MONOKAI_CYAN)
    } else if stretch_status.is_some_and(|view| {
        matches!(
            view.status,
            crate::playback::diagnostics::StretchStatus::Ready { info, .. }
                if (info.time_ratio - 1.0).abs() >= f64::EPSILON
        )
    }) {
        base_style().fg(MONOKAI_GREEN)
    } else if repeated
        || stretch_status.is_some_and(|view| {
            matches!(
                view.status,
                crate::playback::diagnostics::StretchStatus::Ready { info, .. }
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
mod tests;
