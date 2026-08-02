use crate::LoopBrowser;
use cmrt_loop_domain::loop_wav_analysis::LoopWavKind;
use cmrt_loop_domain::loop_waveform::{LoopWaveform, WaveformDisplayScale, SILENCE_DB_TENTHS};
use cmrt_tui_core::theme::{cursor_highlight_style, MONOKAI_GRAY};
use ratatui::{style::Color, text::Span};

use cmrt_tui_core::status::base_style;

mod playhead;

pub use playhead::{append_header_cell, title};

const GLYPHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const BLUE_HUE: f32 = 215.0;
const GREEN_HUE: f32 = 135.0;
const ORANGE_HUE: f32 = 30.0;
const ONE_SEMITONE_OCTAVES: f32 = 1.0 / 12.0;
const SIX_SEMITONES_OCTAVES: f32 = 0.5;

pub fn render_cell(
    browser: &LoopBrowser,
    track: usize,
    measure: usize,
    cell_width: usize,
    measure_seconds: f64,
) -> Vec<Span<'static>> {
    let Some((start, clip)) = browser.clip_at(track, measure) else {
        return vec![Span::styled("·", base_style())];
    };
    let Some(waveform) = browser.waveform_for_wav(&clip.wav) else {
        return vec![Span::styled("?", base_style())];
    };
    if waveform.is_empty() || clip.span_measures == 0 {
        return vec![Span::styled("?", base_style())];
    }

    let output_len = clip.span_measures.saturating_mul(cell_width);
    let output_offset = measure.saturating_sub(start).saturating_mul(cell_width);
    if output_len == 0 || output_offset >= output_len {
        return vec![Span::styled("?", base_style())];
    }
    let waveform_output_len = browser
        .analysis_for_wav(&clip.wav)
        .map_or(output_len, |analysis| {
            waveform_output_columns(
                analysis.kind,
                analysis.duration_seconds,
                clip.span_measures,
                measure_seconds,
                output_len,
            )
        });
    let scale = browser.waveform_display_scale();
    (0..cell_width)
        .map(|column| {
            render_timeline_column(waveform, scale, output_offset + column, waveform_output_len)
        })
        .collect()
}

fn waveform_output_columns(
    kind: LoopWavKind,
    duration_seconds: f64,
    span_measures: usize,
    measure_seconds: f64,
    output_len: usize,
) -> usize {
    if kind != LoopWavKind::OneShot || output_len == 0 {
        return output_len;
    }
    let span_seconds = measure_seconds * span_measures as f64;
    if !duration_seconds.is_finite()
        || duration_seconds <= 0.0
        || !span_seconds.is_finite()
        || span_seconds <= 0.0
    {
        return output_len;
    }
    (duration_seconds / span_seconds * output_len as f64)
        .ceil()
        .clamp(1.0, output_len as f64) as usize
}

fn silent_column() -> Span<'static> {
    Span::styled("·", base_style().fg(MONOKAI_GRAY))
}

fn render_timeline_column(
    waveform: &LoopWaveform,
    scale: WaveformDisplayScale,
    output_column: usize,
    waveform_output_len: usize,
) -> Span<'static> {
    if output_column >= waveform_output_len {
        silent_column()
    } else {
        render_column(waveform, scale, output_column, waveform_output_len)
    }
}

fn render_column(
    waveform: &LoopWaveform,
    scale: WaveformDisplayScale,
    output_column: usize,
    output_len: usize,
) -> Span<'static> {
    let source_start = output_column.saturating_mul(waveform.len()) / output_len;
    let source_end = (output_column + 1)
        .saturating_mul(waveform.len())
        .div_ceil(output_len)
        .max(source_start + 1)
        .min(waveform.len());
    let rms = aggregate_rms(&waveform.rms_db_tenths[source_start..source_end]);
    if rms <= SILENCE_DB_TENTHS {
        return silent_column();
    }
    let flux = waveform.spectral_flux[source_start..source_end]
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
    let rms_rank = scale.rms_rank(rms);
    let flux_rank = scale.flux_rank(flux);
    let glyph_index = (rms_rank * (GLYPHS.len() - 1) as f32).round() as usize;
    let hue = theme_hue(waveform.centroid_motion_octaves());
    let saturation = 0.30 + 0.70 * flux_rank;
    let lightness = 0.32 + 0.40 * rms_rank;
    Span::styled(
        GLYPHS[glyph_index].to_string(),
        base_style().fg(hsl_to_rgb(hue, saturation, lightness)),
    )
}

pub fn append_cell(
    target: &mut Vec<Span<'static>>,
    source: &[Span<'static>],
    muted: bool,
    cursor: bool,
    playback_beat: Option<crate::playback::position::PlaybackBeat>,
    measure: usize,
    width: usize,
) {
    let mut columns = Vec::with_capacity(width);
    for span in source {
        for character in span.content.chars() {
            columns.push((character, span.style));
            if columns.len() == width {
                break;
            }
        }
        if columns.len() == width {
            break;
        }
    }
    let active = playhead::active_columns(playback_beat, measure, width);
    for column in 0..width {
        let (character, source_style) = columns.get(column).copied().unwrap_or((' ', base_style()));
        let mut style = if muted {
            base_style().fg(MONOKAI_GRAY)
        } else {
            source_style
        };
        if cursor {
            style = cursor_highlight_style(style);
        }
        if active.as_ref().is_some_and(|range| range.contains(&column)) {
            style = playhead::marker_style(style);
        }
        target.push(Span::styled(character.to_string(), style));
    }
}

fn aggregate_rms(values: &[i16]) -> i16 {
    let mut energy = 0.0_f64;
    let mut audible = false;
    for &value in values {
        if value > SILENCE_DB_TENTHS {
            audible = true;
        }
        energy += 10.0_f64.powf(f64::from(value) / 100.0);
    }
    if !audible || values.is_empty() {
        SILENCE_DB_TENTHS
    } else {
        (100.0 * (energy / values.len() as f64).log10())
            .round()
            .clamp(f64::from(SILENCE_DB_TENTHS), 240.0) as i16
    }
}

fn theme_hue(motion_octaves: f32) -> f32 {
    if motion_octaves < ONE_SEMITONE_OCTAVES {
        BLUE_HUE
    } else if motion_octaves < SIX_SEMITONES_OCTAVES {
        GREEN_HUE
    } else {
        ORANGE_HUE
    }
}

fn hsl_to_rgb(hue_degrees: f32, saturation: f32, lightness: f32) -> Color {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue = hue_degrees / 60.0;
    let intermediate = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue as u8 {
        0 => (chroma, intermediate, 0.0),
        1 => (intermediate, chroma, 0.0),
        2 => (0.0, chroma, intermediate),
        3 => (0.0, intermediate, chroma),
        4 => (intermediate, 0.0, chroma),
        _ => (chroma, 0.0, intermediate),
    };
    let offset = lightness - chroma / 2.0;
    Color::Rgb(
        ((red + offset) * 255.0).round() as u8,
        ((green + offset) * 255.0).round() as u8,
        ((blue + offset) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests;
