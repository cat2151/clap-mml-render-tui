//! 音色一覧の計測値の列（Size / Load）。

use ratatui::{layout::Alignment, style::Style, text::Line, widgets::Cell};

use cmrt_tui_core::{patch_load::PatchLoadMeasurement, theme::MONOKAI_PINK};

/// Size 列。`.sfz` が読む sample の総容量で、重い（先読みしない）ものだけ色を付ける。
pub(super) fn sample_size_cell(measurement: Option<&PatchLoadMeasurement>) -> Cell<'static> {
    let label = sample_size_label(measurement);
    let line = Line::from(label).alignment(Alignment::Right);
    if measurement.is_some_and(PatchLoadMeasurement::is_heavy_offline_load) {
        Cell::from(line.style(Style::default().fg(MONOKAI_PINK)))
    } else {
        Cell::from(line)
    }
}

/// Size 列の表記。`.sfz` 以外と集計できなかったものは空欄。
pub(super) fn sample_size_label(measurement: Option<&PatchLoadMeasurement>) -> String {
    match measurement.and_then(|measurement| measurement.sfz_sample_bytes) {
        None => String::new(),
        Some(bytes) if bytes < 1_000_000 => "<1MB".to_string(),
        Some(bytes) => format!("{}MB", bytes / 1_000_000),
    }
}

/// Load 列の表記。2 回目の読み込み時間を短く出し、未計測なら `-`。
pub fn load_time_label(measurement: Option<&PatchLoadMeasurement>) -> String {
    measurement
        .and_then(|measurement| measurement.second_load_ms)
        .map_or_else(|| "-".to_string(), format_load_time)
}

pub(super) fn format_load_time(milliseconds: u64) -> String {
    match milliseconds {
        0..=99 => format!("{milliseconds}ms"),
        100..=999 => format!("0.{}s", milliseconds / 100),
        _ => format!("{}s", milliseconds / 1_000),
    }
}
