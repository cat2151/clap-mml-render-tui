use std::collections::HashMap;

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{cell_style, keep_visible, stretch_label};
use crate::loop_browser_metadata::LoopWavId;
use crate::loop_browser_track_grid::LoopTrackClip;
use crate::tui::loop_browser::playback::diagnostics::LoopStretchDiagnostics;
use crate::tui::loop_browser::{LoopBrowser, LoopPlaybackClip};
use crate::tui::ui::status::base_style;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GRAY};

const TRACK_WIDTH: usize = 7;
const MEASURE_WIDTH: usize = 14;
const WAV_WIDTH: usize = 24;
const ANALYSIS_WIDTH: usize = 32;
const CATEGORY_WIDTH: usize = 16;
const STRETCH_MIN_WIDTH: usize = 10;

#[derive(Clone, Copy)]
struct ColumnWidths {
    track: usize,
    measures: usize,
    wav: usize,
    analysis: usize,
    category: usize,
}

impl ColumnWidths {
    fn fit(area_width: usize, show_measures: bool, rows: &[DisplayRow]) -> Self {
        let mut widths = Self {
            track: content_width(
                "track",
                rows.iter().map(|row| format!("T{}", row.track + 1)),
                4,
                TRACK_WIDTH,
            ),
            measures: if show_measures {
                content_width(
                    "meas",
                    rows.iter().map(|row| row.measures.clone()),
                    6,
                    MEASURE_WIDTH,
                )
            } else {
                0
            },
            wav: content_width("WAV", rows.iter().map(|row| row.wav.clone()), 8, WAV_WIDTH),
            analysis: content_width(
                "analysis",
                rows.iter().map(|row| row.analysis.clone()),
                10,
                ANALYSIS_WIDTH,
            ),
            category: content_width(
                "category",
                rows.iter().map(|row| row.category.clone()),
                7,
                CATEGORY_WIDTH,
            ),
        };
        let target = area_width.saturating_sub(STRETCH_MIN_WIDTH);
        let mut total = widths.total();
        shrink_column(&mut widths.wav, 8, &mut total, target);
        shrink_column(&mut widths.analysis, 10, &mut total, target);
        shrink_column(&mut widths.category, 7, &mut total, target);
        shrink_column(
            &mut widths.measures,
            usize::from(show_measures) * 6,
            &mut total,
            target,
        );
        shrink_column(&mut widths.track, 4, &mut total, target);
        widths
    }

    fn total(self) -> usize {
        self.track + self.measures + self.wav + self.analysis + self.category
    }
}

fn content_width(
    header: &str,
    values: impl Iterator<Item = String>,
    minimum: usize,
    maximum: usize,
) -> usize {
    values
        .map(|value| Line::from(value).width())
        .chain(std::iter::once(Line::from(header).width()))
        .max()
        .unwrap_or(minimum)
        .saturating_add(2)
        .clamp(minimum, maximum)
}

fn shrink_column(column: &mut usize, minimum: usize, total: &mut usize, target: usize) {
    let reduction = total
        .saturating_sub(target)
        .min(column.saturating_sub(minimum));
    *column -= reduction;
    *total -= reduction;
}

struct UsedWavRow {
    track: usize,
    wav: LoopWavId,
    clip: LoopTrackClip,
    playback_clip: LoopPlaybackClip,
    measures: Vec<usize>,
}

struct DisplayRow {
    track: usize,
    measures: String,
    wav: String,
    analysis: String,
    category: String,
    stretch: String,
    style: Style,
    selected: bool,
}

pub(super) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: &mut LoopBrowser,
    diagnostics: &LoopStretchDiagnostics,
    target_bpm: f64,
    focused: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (rows, show_measures) = display_rows(browser, diagnostics, target_bpm, focused);
    let widths = ColumnWidths::fit(usize::from(area.width), show_measures, &rows);
    let visible_rows = usize::from(area.height.saturating_sub(1));
    let selected = rows.iter().position(|row| row.selected);
    if visible_rows > 0 {
        if let Some(selected) = selected {
            keep_visible(selected, visible_rows, &mut browser.used_wav_scroll);
        }
        browser.used_wav_scroll = browser
            .used_wav_scroll
            .min(rows.len().saturating_sub(visible_rows));
    } else {
        browser.used_wav_scroll = 0;
    }

    let mut lines = Vec::with_capacity(visible_rows.saturating_add(1));
    lines.push(header(show_measures, widths));
    lines.extend(
        rows.iter()
            .skip(browser.used_wav_scroll)
            .take(visible_rows)
            .map(|row| render_row(row, show_measures, widths)),
    );
    frame.render_widget(Paragraph::new(lines).style(base_style()), area);
}

fn display_rows(
    browser: &LoopBrowser,
    diagnostics: &LoopStretchDiagnostics,
    target_bpm: f64,
    focused: bool,
) -> (Vec<DisplayRow>, bool) {
    let selected = browser
        .displayed_clip_at(browser.track_cursor, browser.measure_cursor)
        .map(|clip| (browser.track_cursor, clip.wav.lookup_key()));
    let mut rows = Vec::new();
    let mut show_measures = false;
    for (track, cells) in browser.track_grid().iter().enumerate() {
        let mut track_rows = Vec::<UsedWavRow>::new();
        let mut indices = HashMap::<(String, String), usize>::new();
        for (measure, clip) in cells
            .iter()
            .enumerate()
            .filter_map(|(measure, clip)| clip.as_ref().map(|clip| (measure, clip)))
        {
            let key = clip.wav.lookup_key();
            if let Some(index) = indices.get(&key).copied() {
                track_rows[index].measures.push(measure);
                if track_rows[index].clip.is_previous() && !clip.is_previous() {
                    track_rows[index].clip = clip.clone();
                    track_rows[index].playback_clip = browser.playback_clip(clip);
                }
            } else {
                indices.insert(key, track_rows.len());
                track_rows.push(UsedWavRow {
                    track,
                    wav: clip.wav.clone(),
                    clip: clip.clone(),
                    playback_clip: browser.playback_clip(clip),
                    measures: vec![measure],
                });
            }
        }
        show_measures |= track_rows.len() > 1;
        rows.extend(track_rows);
    }

    let rows = rows
        .into_iter()
        .map(|row| {
            let analysis = browser
                .analysis_for_wav(&row.wav)
                .map(crate::loop_wav_analysis::format_analysis)
                .unwrap_or_else(|| "[analysis unavailable]".to_string());
            let category = row
                .playback_clip
                .category
                .clone()
                .unwrap_or_else(|| "none".to_string());
            let stretch_status = diagnostics.status_for(&row.playback_clip, target_bpm);
            let exceeds_limits = row.playback_clip.source_bpm().is_some_and(|source_bpm| {
                crate::loop_time_stretch::exceeds_time_ratio_limits(source_bpm, target_bpm)
            });
            let style = if !browser.track_is_audible(row.track) {
                base_style().fg(MONOKAI_GRAY)
            } else {
                cell_style(stretch_status.as_ref(), false, exceeds_limits)
            };
            let selected = focused
                && selected.as_ref().is_some_and(|(track, key)| {
                    *track == row.track && *key == row.wav.lookup_key()
                });
            DisplayRow {
                track: row.track,
                measures: row
                    .measures
                    .iter()
                    .map(|measure| (measure + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                wav: row
                    .wav
                    .path()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| row.wav.relative.clone()),
                analysis,
                category,
                stretch: stretch_label(
                    row.playback_clip.source_bpm(),
                    target_bpm,
                    row.clip.span_measures,
                    row.playback_clip.meter_numerator,
                    row.playback_clip.meter_denominator,
                    stretch_status.as_ref(),
                    exceeds_limits,
                ),
                style,
                selected,
            }
        })
        .collect();
    (rows, show_measures)
}

fn header(show_measures: bool, widths: ColumnWidths) -> Line<'static> {
    let mut spans = vec![Span::styled(
        fit_column("track", widths.track),
        base_style().fg(MONOKAI_CYAN),
    )];
    if show_measures {
        spans.push(Span::styled(
            fit_column("meas", widths.measures),
            base_style().fg(MONOKAI_CYAN),
        ));
    }
    spans.extend([
        Span::styled(fit_column("WAV", widths.wav), base_style().fg(MONOKAI_CYAN)),
        Span::styled(
            fit_column("analysis", widths.analysis),
            base_style().fg(MONOKAI_CYAN),
        ),
        Span::styled(
            fit_column("category", widths.category),
            base_style().fg(MONOKAI_CYAN),
        ),
        Span::styled("stretch", base_style().fg(MONOKAI_CYAN)),
    ]);
    Line::from(spans)
}

fn render_row(row: &DisplayRow, show_measures: bool, widths: ColumnWidths) -> Line<'static> {
    let mut text = fit_column(&format!("T{}", row.track + 1), widths.track);
    if show_measures {
        text.push_str(&fit_column(&row.measures, widths.measures));
    }
    text.push_str(&fit_column(&row.wav, widths.wav));
    text.push_str(&fit_column(&row.analysis, widths.analysis));
    text.push_str(&fit_column(&row.category, widths.category));
    text.push_str(&row.stretch);
    let style = if row.selected {
        cursor_highlight_style(row.style)
    } else {
        row.style
    };
    Line::styled(text, style)
}

fn fit_column(text: &str, width: usize) -> String {
    let mut output = String::new();
    for character in text.chars() {
        let mut candidate = output.clone();
        candidate.push(character);
        if Line::from(candidate.as_str()).width() > width {
            break;
        }
        output.push(character);
    }
    let used = Line::from(output.as_str()).width();
    output.extend(std::iter::repeat_n(' ', width.saturating_sub(used)));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_column_is_only_added_when_a_track_has_multiple_wavs() {
        let single = header(false, ColumnWidths::fit(90, false, &[])).to_string();
        let multiple = header(true, ColumnWidths::fit(90, true, &[])).to_string();
        assert!(!single.contains("meas"));
        assert!(multiple.contains("meas"));
    }

    #[test]
    fn narrow_layout_reserves_room_for_stretch_status() {
        let widths = ColumnWidths::fit(50, true, &[]);
        assert!(widths.total() + STRETCH_MIN_WIDTH <= 50);
        assert!(widths.track >= 4);
        assert!(widths.measures >= 6);
        assert!(widths.wav >= 8);
        assert!(widths.analysis >= 10);
        assert!(widths.category >= 7);
    }

    #[test]
    fn columns_fit_wide_characters_by_terminal_width() {
        assert_eq!(fit_column("ドラム", 5), "ドラ ");
    }
}
