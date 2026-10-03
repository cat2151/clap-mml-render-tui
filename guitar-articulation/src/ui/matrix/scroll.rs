//! matrix を横へ送る範囲。カーソルを追う間は右端まで送らず、演奏中は鳴っている列を真ん中に置く。

/// 1 列の桁（記号 1 つ + 空白）。
const COLUMN_WIDTH: usize = 2;

/// 見出しの右に入る列の数。
fn fitting_columns(width: u16, label_width: usize) -> usize {
    (usize::from(width).saturating_sub(label_width) / COLUMN_WIDTH).max(1)
}

/// `anchor` の列が必ず見えるように横へずらした、描く列の範囲。`anchor` は右端に来るまで送らない。
pub(in crate::ui) fn visible_columns(
    anchor: usize,
    column_count: usize,
    width: u16,
    label_width: usize,
) -> std::ops::Range<usize> {
    let fit = fitting_columns(width, label_width);
    let first = (anchor + 1).saturating_sub(fit);
    first..column_count.min(first + fit)
}

/// `center` の列を真ん中に置いた、描く列の範囲。先頭と末尾では端で止め、左右に空白の列を作らない。
pub(super) fn centered_columns(
    center: usize,
    column_count: usize,
    width: u16,
    label_width: usize,
) -> std::ops::Range<usize> {
    let fit = fitting_columns(width, label_width);
    let first = center
        .saturating_sub(fit / 2)
        .min(column_count.saturating_sub(fit));
    first..column_count.min(first + fit)
}
