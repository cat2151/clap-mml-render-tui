//! 1 行のコード表記を、chord ごとの元の文字列上の範囲へ切り分ける。
//!
//! 範囲は chord2mml の CST が `ParsedItem::Chord { source_range }` として既に持っているので、
//! パーサはここでも書かない（[`crate::cursor_sounding_unit`] と同じ材料・同じ読み方）。
//! あちらとの違いは「カーソル 1 点が乗っている 1 つ」ではなく行の chord を順に全部返すことと、
//! MML 経路のフォールバックを持たないこと。

use std::ops::Range;

use chord2mml_core::ParsedItem;

/// 1 行に含まれる chord の、元の文字列上のバイト範囲を出現順に返す。
///
/// Key や directive（`Key=C` `drop2` など）の範囲は含まない。chord2mml が読めない文字列は
/// 空を返す（`Err` にしない。呼び出し側は「行全体を 1 つとして扱う」へ倒す）。
/// 範囲は呼び出し側の文字列を指す（dialect の書き換えが起きる `ii` のような綴りでも）。
pub fn chord_source_ranges(line: &str) -> Vec<Range<usize>> {
    let Ok(parsed) = chord2mml_core::parse(line) else {
        return Vec::new();
    };
    // 全体が MML へ変換できない行は、切り出した 1 つも鳴らせない。
    // `cursor_sounding_unit` が MML 経路へ落とすのと同じ判定にそろえる。
    if parsed.to_mml().is_err() {
        return Vec::new();
    }
    parsed
        .items()
        .iter()
        .filter_map(|item| match item {
            ParsedItem::Chord {
                source_range: Some(range),
                ..
            } => Some(range.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
