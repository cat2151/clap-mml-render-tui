use cmrt_loop_domain::loop_waveform::WAVEFORM_BINS_PER_MEASURE;

/// これ以上詰めると WAV 名も拍ヘッダも読めなくなる下限。
/// ここまで詰めても入らないぶんだけ横スクロールへ回す。
pub const MIN_CELL_WIDTH: usize = 8;

/// 1 文字 = 解析 1 bin = 32 分音符。これ以上広げても分解能は増えない。
pub const MAX_CELL_WIDTH: usize = WAVEFORM_BINS_PER_MEASURE;

pub fn keep_visible(cursor: usize, visible: usize, scroll: &mut usize) {
    if cursor < *scroll {
        *scroll = cursor;
    } else if cursor >= *scroll + visible {
        *scroll = cursor + 1 - visible;
    }
}

/// 表示中の小節が全部入る最大のセル幅。
///
/// 小節数が少ないループほど広いセルになり、上限まで広げると波形 1 文字が
/// 解析 1 bin（32 分音符）に 1 対 1 で対応する。
pub fn measure_cell_width(available: usize, measure_count: usize) -> usize {
    (available / measure_count.max(1)).clamp(MIN_CELL_WIDTH, MAX_CELL_WIDTH)
}

/// セルが狭いと `fit` が "measure 16" を "measure " まで削って小節番号ごと消してしまう。
/// 番号だけは必ず残るよう、入らないときは短い形へ落とす。
pub fn measure_label(measure: usize, width: usize) -> String {
    let full = format!("measure {}", measure + 1);
    if full.chars().count() <= width {
        full
    } else {
        format!("M{}", measure + 1)
    }
}

pub fn fit(text: &str, width: usize) -> String {
    let mut output = text.chars().take(width).collect::<String>();
    let count = output.chars().count();
    output.extend(std::iter::repeat_n(' ', width.saturating_sub(count)));
    output
}

#[cfg(test)]
mod tests;
