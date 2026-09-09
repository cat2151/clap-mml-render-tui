//! 桁数を合わせるための小さな整形ヘルパ。
//!
//! 幅は**文字数ではなく表示幅**で数える。section 名に全角が入ると
//! `chars().count()` では列がずれるため。

use ratatui::text::Span;

/// 切り詰めたことを示す 1 文字。DAW の chord track（`daw/src/ui/grid.rs` の
/// `fit_cell_text`）と同じ見せ方に揃えてある。
pub(crate) const ELLIPSIS: char = '…';

/// 端末上でその文字列が占める桁数。
pub(crate) fn display_width(text: &str) -> usize {
    Span::raw(text).width()
}

/// `width` 桁ちょうどに収める。溢れたら末尾を [`ELLIPSIS`] にする。
///
/// 「切れていること自体が読めない」のが最悪なので、切り詰めは必ず記号で示す。
pub(crate) fn fit_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let text_width = display_width(text);
    if text_width <= width {
        return format!("{text}{}", " ".repeat(width - text_width));
    }

    let mut fitted = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let char_width = display_width(&ch.to_string());
        if used + char_width > width - 1 {
            break;
        }
        fitted.push(ch);
        used += char_width;
    }
    fitted.push(ELLIPSIS);
    used += 1;
    fitted.push_str(&" ".repeat(width.saturating_sub(used)));
    fitted
}

/// `width` 桁の右詰め。溢れたらそのまま返す（数値を削るくらいならはみ出す）。
pub(crate) fn right_align(text: &str, width: usize) -> String {
    let text_width = display_width(text);
    format!("{}{text}", " ".repeat(width.saturating_sub(text_width)))
}
