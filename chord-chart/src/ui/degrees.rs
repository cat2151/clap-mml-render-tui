//! degrees の 1 行を、chord カーソルの当たっている 1 つだけ反転して描く。
//!
//! **この crate は degrees を解釈しない**（ADR 0020）ので、どこからどこまでが 1 つの
//! chord かは自分では決められない。範囲は app 側の glue が
//! [`ChordChartScreen::set_chord_ranges`](crate::ChordChartScreen::set_chord_ranges)
//! で書き戻した写しから来る。ここがやるのは
//! **バイト位置の範囲を、端末の桁へ読み替えて span を割る**ことだけ。
//!
//! 範囲が無い（写しがまだ届いていない / 読めない degrees で 0 件）ときは
//! 1 つの span のまま。反転が出ないだけで、行は打ったとおりに出る。

use std::ops::Range;

use ratatui::{style::Modifier, text::Span};

use super::text::{display_width, fit_width};

/// degrees 1 行ぶんの span。`highlight` はその degrees の**バイト位置**。
///
/// 幅に収まらない行は [`fit_width`] が `…` で切る。切られた向こう側にある chord は
/// **反転が見えないだけ**で、桁がずれることも panic することもない（切った文字列を
/// 桁で割るので、範囲が画面外なら中央の span が空になる）。
pub(super) fn degrees_spans(
    degrees: &str,
    width: usize,
    highlight: Option<Range<usize>>,
    style: ratatui::style::Style,
) -> Vec<Span<'static>> {
    let fitted = fit_width(degrees, width);
    let Some(columns) = highlight.and_then(|range| highlight_columns(degrees, range)) else {
        return vec![Span::styled(fitted, style)];
    };
    let (before, rest) = split_at_column(&fitted, columns.start);
    let (chord, after) = split_at_column(&rest, columns.end - columns.start);
    [
        (before, style),
        (chord, style.add_modifier(Modifier::REVERSED)),
        (after, style),
    ]
    .into_iter()
    .filter(|(text, _)| !text.is_empty())
    .map(|(text, style)| Span::styled(text, style))
    .collect()
}

/// バイト位置の範囲を、行頭からの**表示桁**の範囲へ読み替える。
///
/// 文字境界で切れていない範囲（写しが古いときに起きうる）は `None`。
/// 桁数を数えるのに `&degrees[..n]` を使うので、ここで弾かないと panic する。
fn highlight_columns(degrees: &str, range: Range<usize>) -> Option<Range<usize>> {
    if range.start >= range.end {
        return None;
    }
    let start = display_width(degrees.get(..range.start)?);
    let end = display_width(degrees.get(..range.end)?);
    Some(start..end)
}

/// 表示幅で `column` 桁のところで割る。全角の途中では割らない
/// （割れない文字は後ろ側へ回す）。`column` が行の幅を超えていれば後ろ側は空。
fn split_at_column(text: &str, column: usize) -> (String, String) {
    let mut head = String::new();
    let mut used = 0;
    let mut chars = text.chars().peekable();
    while let Some(&ch) = chars.peek() {
        let char_width = display_width(&ch.to_string());
        if used + char_width > column {
            break;
        }
        head.push(ch);
        used += char_width;
        chars.next();
    }
    (head, chars.collect())
}

#[cfg(test)]
mod tests;
