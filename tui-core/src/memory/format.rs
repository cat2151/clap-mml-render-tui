//! 計測結果を help overlay の行へ整形する。

use ratatui::text::Line;

use super::{MemoryReading, MemorySnapshot};

/// 数値部の表示桁数。物理メモリの現実的な上限（1023.99 GB）が収まる幅。
const VALUE_WIDTH: usize = 10;
const MEASURING: &str = "計測中";
const UNAVAILABLE: &str = "取得不可";

#[cfg(test)]
mod tests;

pub(super) fn overlay_lines(reading: MemoryReading) -> Vec<Line<'static>> {
    let (total, available) = match reading {
        MemoryReading::Ready(MemorySnapshot {
            total_working_set_bytes,
            os_available_bytes,
        }) => (
            format_bytes(total_working_set_bytes),
            format_bytes(os_available_bytes),
        ),
        MemoryReading::Measuring => (MEASURING.to_string(), MEASURING.to_string()),
        MemoryReading::Unavailable => (UNAVAILABLE.to_string(), UNAVAILABLE.to_string()),
    };

    // 区切りの空行まで含めて返し、呼び出し側は既存のヘルプ行の前に置くだけでよくする。
    vec![Line::from(memory_text(&total, &available)), Line::from("")]
}

/// 数値部を固定幅で右詰めする。
///
/// 値の桁数や計測状態でこの行の幅が変わると、`ui::centered_text_block_rect` が
/// 返す overlay の枠幅が open のたびに伸縮してしまうため。
fn memory_text(total: &str, available: &str) -> String {
    format!(
        " 実メモリ合計 {}   OS空き {}",
        pad_left(total, VALUE_WIDTH),
        pad_left(available, VALUE_WIDTH),
    )
}

/// 全角を含むので、文字数ではなく表示桁数で右詰めする。
fn pad_left(text: &str, width: usize) -> String {
    let padding = width.saturating_sub(Line::from(text).width());
    format!("{}{text}", " ".repeat(padding))
}

const KIB: f64 = 1024.0;

fn format_bytes(bytes: u64) -> String {
    let mib = bytes as f64 / (KIB * KIB);
    if mib < KIB {
        return format!("{mib:.0} MB");
    }
    let gib = mib / KIB;
    if gib < KIB {
        return format!("{gib:.2} GB");
    }
    format!("{:.2} TB", gib / KIB)
}
