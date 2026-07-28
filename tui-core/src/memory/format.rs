//! 計測結果を help overlay の行へ整形する。

use ratatui::text::Line;

use super::{MemoryReading, MemorySnapshot, ProcessMemory};

/// 数値部の表示桁数。物理メモリの現実的な上限（1023.99 GB）が収まる幅。
const VALUE_WIDTH: usize = 10;
/// 内訳行のプロセス名カラム幅。既知の最長名 `clap-mml-realtime-play-server.exe` に合わせる。
const NAME_WIDTH: usize = 33;
/// 内訳行の字下げ。合計行より内側にあることを示す。
const DETAIL_INDENT: &str = "   ";
/// 内訳に出す行数の上限。
///
/// サーバを手動で複数常駐させると内訳がいくらでも伸び、help 本体を画面外へ
/// 押し出してしまうため、ここで打ち切って残りは 1 行にまとめる。
const MAX_DETAIL_LINES: usize = 6;
const MEASURING: &str = "計測中";
const UNAVAILABLE: &str = "取得不可";

#[cfg(test)]
mod tests;

pub(super) fn overlay_lines(reading: MemoryReading) -> Vec<Line<'static>> {
    let (total, available, processes) = match reading {
        MemoryReading::Ready(MemorySnapshot {
            processes,
            total_working_set_bytes,
            os_available_bytes,
        }) => (
            format_bytes(total_working_set_bytes),
            format_bytes(os_available_bytes),
            processes,
        ),
        MemoryReading::Measuring => (MEASURING.to_string(), MEASURING.to_string(), Vec::new()),
        MemoryReading::Unavailable => {
            (UNAVAILABLE.to_string(), UNAVAILABLE.to_string(), Vec::new())
        }
    };

    let mut lines = vec![Line::from(memory_text(&total, &available))];
    lines.extend(detail_lines(&processes).into_iter().map(Line::from));
    // 区切りの空行まで含めて返し、呼び出し側は既存のヘルプ行の前に置くだけでよくする。
    lines.push(Line::from(""));

    lines
}

/// プロセスごとの内訳行。計測が済んでいないときは空（合計行だけになる）。
fn detail_lines(processes: &[ProcessMemory]) -> Vec<String> {
    if processes.len() <= MAX_DETAIL_LINES {
        return processes.iter().map(detail_text).collect();
    }

    // 打ち切った行があるぶん、まとめ行のぶんだけ手前で切る。
    let shown = MAX_DETAIL_LINES - 1;
    let mut lines: Vec<String> = processes[..shown].iter().map(detail_text).collect();
    let omitted = processes.len() - shown;
    let omitted_bytes = processes[shown..]
        .iter()
        .map(|process| process.working_set_bytes)
        .fold(0u64, u64::saturating_add);
    lines.push(detail_row(
        &format!("他 {omitted} プロセス"),
        &format_bytes(omitted_bytes),
    ));

    lines
}

fn detail_text(process: &ProcessMemory) -> String {
    detail_row(&process.name, &format_bytes(process.working_set_bytes))
}

fn detail_row(name: &str, value: &str) -> String {
    format!(
        "{DETAIL_INDENT}{}{}",
        pad_right(name, NAME_WIDTH),
        pad_left(value, VALUE_WIDTH),
    )
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

/// 左詰め。幅を超える名前は切り詰め、内訳行の幅が名前の長さで動かないようにする。
fn pad_right(text: &str, width: usize) -> String {
    let text = truncate_to_width(text, width);
    let padding = width.saturating_sub(Line::from(text.as_str()).width());
    format!("{text}{}", " ".repeat(padding))
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if Line::from(text).width() <= width {
        return text.to_string();
    }

    let mut truncated = String::new();
    for character in text.chars() {
        let mut candidate = truncated.clone();
        candidate.push(character);
        if Line::from(candidate.as_str()).width() > width {
            break;
        }
        truncated = candidate;
    }

    truncated
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
