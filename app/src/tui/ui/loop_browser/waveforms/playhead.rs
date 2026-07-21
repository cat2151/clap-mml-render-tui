use std::ops::Range;

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::tui::loop_browser::playback::position::PlaybackBeat;

use super::super::super::status::base_style;
use crate::ui_theme::MONOKAI_CYAN;

pub(super) const ACTIVE_BEAT_BG: Color = Color::Rgb(78, 65, 35);

pub(in crate::tui::ui::loop_browser) fn title(position: Option<PlaybackBeat>) -> String {
    position.map_or_else(
        || " [RMS / SPECTRAL MOTION] ".to_string(),
        |position| {
            format!(
                " [RMS / SPECTRAL MOTION | PLAY M{} B{}] ",
                position.measure + 1,
                position.beat + 1
            )
        },
    )
}

pub(in crate::tui::ui::loop_browser) fn append_header_cell(
    target: &mut Vec<Span<'static>>,
    measure: usize,
    beats_per_measure: usize,
    position: Option<PlaybackBeat>,
    width: usize,
) {
    let labels = beat_labels(width, beats_per_measure);
    let active = position
        .filter(|position| position.measure == measure)
        .map(|position| beat_columns(position.beat, position.beats_per_measure, width));
    for (column, label) in labels.into_iter().enumerate() {
        let mut style = base_style().fg(MONOKAI_CYAN);
        if active.as_ref().is_some_and(|range| range.contains(&column)) {
            style = marker_style(style);
        }
        target.push(Span::styled(label.to_string(), style));
    }
}

pub(super) fn active_columns(
    position: Option<PlaybackBeat>,
    measure: usize,
    width: usize,
) -> Option<Range<usize>> {
    position
        .filter(|position| position.measure == measure)
        .map(|position| beat_columns(position.beat, position.beats_per_measure, width))
}

pub(super) fn marker_style(style: Style) -> Style {
    style.bg(ACTIVE_BEAT_BG).add_modifier(Modifier::BOLD)
}

fn beat_columns(beat: usize, beats_per_measure: usize, width: usize) -> Range<usize> {
    if beats_per_measure == 0 || width == 0 {
        return 0..0;
    }
    let beat = beat.min(beats_per_measure - 1);
    let start = beat.saturating_mul(width) / beats_per_measure;
    let proportional_end = (beat + 1).saturating_mul(width) / beats_per_measure;
    let end = proportional_end.max(start + 1).min(width);
    start.min(width)..end
}

fn beat_labels(width: usize, beats_per_measure: usize) -> Vec<char> {
    let mut labels = vec![' '; width];
    if beats_per_measure == 0 {
        return labels;
    }
    for beat in 0..beats_per_measure {
        let start = beat.saturating_mul(width) / beats_per_measure;
        for (offset, character) in (beat + 1).to_string().chars().enumerate() {
            let column = start + offset;
            if column < width && labels[column] == ' ' {
                labels[column] = character;
            }
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_four_uses_four_columns_per_beat() {
        assert_eq!(beat_columns(0, 4, 16), 0..4);
        assert_eq!(beat_columns(1, 4, 16), 4..8);
        assert_eq!(beat_columns(2, 4, 16), 8..12);
        assert_eq!(beat_columns(3, 4, 16), 12..16);
        assert_eq!(
            beat_labels(16, 4).into_iter().collect::<String>(),
            "1   2   3   4   "
        );
    }

    #[test]
    fn three_four_distributes_all_sixteen_columns() {
        assert_eq!(beat_columns(0, 3, 16), 0..5);
        assert_eq!(beat_columns(1, 3, 16), 5..10);
        assert_eq!(beat_columns(2, 3, 16), 10..16);
    }

    #[test]
    fn title_reports_position_even_when_its_measure_is_not_visible() {
        let title = title(Some(PlaybackBeat {
            measure: 2,
            beat: 1,
            beats_per_measure: 4,
        }));

        assert!(title.contains("PLAY M3 B2"));
    }
}
