use crate::loop_waveform::{LoopWaveform, WaveformDisplayScale, SILENCE_DB_TENTHS};
use crate::tui::loop_browser::LoopBrowser;
use crate::ui_theme::{cursor_highlight_style, MONOKAI_GRAY};
use ratatui::{style::Color, text::Span};

use super::super::status::base_style;

mod playhead;

pub(super) use playhead::{append_header_cell, title};

const GLYPHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const BLUE_HUE: f32 = 215.0;
const GREEN_HUE: f32 = 135.0;
const ORANGE_HUE: f32 = 30.0;
const ONE_SEMITONE_OCTAVES: f32 = 1.0 / 12.0;
const SIX_SEMITONES_OCTAVES: f32 = 0.5;

pub(super) fn render_cell(
    browser: &LoopBrowser,
    track: usize,
    measure: usize,
    cell_width: usize,
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
    let scale = browser.waveform_display_scale();
    (0..cell_width)
        .map(|column| render_column(waveform, scale, output_offset + column, output_len))
        .collect()
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
        return Span::styled("·", base_style().fg(MONOKAI_GRAY));
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

pub(super) fn append_cell(
    target: &mut Vec<Span<'static>>,
    source: &[Span<'static>],
    muted: bool,
    cursor: bool,
    playback_beat: Option<crate::tui::loop_browser::playback::position::PlaybackBeat>,
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
mod tests {
    use super::*;

    #[test]
    fn motion_thresholds_select_blue_green_and_orange() {
        assert_eq!(theme_hue(0.0), BLUE_HUE);
        assert_eq!(theme_hue(ONE_SEMITONE_OCTAVES), GREEN_HUE);
        assert_eq!(theme_hue(SIX_SEMITONES_OCTAVES), ORANGE_HUE);
    }

    #[test]
    fn aggregate_rms_uses_mean_energy_instead_of_peak() {
        assert_eq!(aggregate_rms(&[-200, -200]), -200);
        assert_eq!(aggregate_rms(&[SILENCE_DB_TENTHS; 2]), SILENCE_DB_TENTHS);
        assert!(aggregate_rms(&[-100, -300]) < -100);
    }

    #[test]
    fn hsl_output_uses_requested_dominant_theme_channel() {
        let Color::Rgb(orange_r, orange_g, orange_b) = hsl_to_rgb(ORANGE_HUE, 0.8, 0.5) else {
            panic!("RGB expected");
        };
        assert!(orange_r > orange_g && orange_g > orange_b);
        let Color::Rgb(_, green, blue) = hsl_to_rgb(BLUE_HUE, 0.8, 0.5) else {
            panic!("RGB expected");
        };
        assert!(blue > green);
    }

    #[test]
    fn active_beat_band_overrides_cursor_background_and_keeps_waveform_foreground() {
        let foreground = Color::Rgb(12, 200, 80);
        let source = vec![Span::styled(
            "abcdefghijklmnop",
            base_style().fg(foreground),
        )];
        let mut rendered = Vec::new();

        append_cell(
            &mut rendered,
            &source,
            false,
            true,
            Some(crate::tui::loop_browser::playback::position::PlaybackBeat {
                measure: 1,
                beat: 2,
                beats_per_measure: 4,
            }),
            1,
            16,
        );

        assert_eq!(rendered.len(), 16);
        for (column, span) in rendered.iter().enumerate() {
            assert_eq!(span.style.fg, Some(foreground));
            if (8..12).contains(&column) {
                assert_eq!(span.style.bg, Some(playhead::ACTIVE_BEAT_BG));
            } else {
                assert_eq!(
                    span.style.bg,
                    Some(crate::ui_theme::cursor_highlight_bg(foreground))
                );
            }
        }
    }
}
