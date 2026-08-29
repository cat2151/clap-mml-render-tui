//! カーソル位置を、そこにある「発音単位」の範囲へ写す。
//!
//! 発音単位の切り出しそのものは本家 mmlabc-to-smf が CST から返す
//! （`sounding_units`）。ここが持つのは MML 経路と chord2mml 経路のどちらを
//! 使うかの選択だけで、MML の文法はひとつも持たない。
//!
//! コード表記の行では、chord2mml の CST が返す元入力上の chord 範囲を使う。

use std::ops::Range;

use chord2mml_core::ParsedItem;
use mmlabc_to_smf::sounding_units::{sounding_units, unit_at};

/// カーソルのある発音単位の範囲。鳴らすものが無ければ `None`。
///
/// `cursor` は行頭からのバイト位置。返る範囲は
///
/// - MML の行なら、その発音単位ひとつ。和音は両方のクォートを含めて 1 つ。
///   休符やコマンド（`o5` `l8` `<` など）の上では `None`。
/// - コード表記の行なら、そのコード表記ひとつ。Key や directive 上では `None`。
///
/// 範囲の終端まで解釈すれば、その単位の音がそのまま鳴る。カーソルの手前で
/// 切らないので、`c4` の途中にカーソルがあっても 4 分音符として鳴る。
pub fn cursor_sounding_unit(line: &str, cursor: usize) -> Option<Range<usize>> {
    if let Ok(parsed) = chord2mml_core::parse(line) {
        if parsed.to_mml().is_ok() {
            debug_assert!(parsed.items().iter().all(|item| !matches!(
                item,
                ParsedItem::Chord {
                    source_range: None,
                    ..
                }
            )));
            return parsed
                .items()
                .iter()
                .find_map(|item| match item {
                    ParsedItem::Chord {
                        source_range: Some(range),
                        ..
                    } if range.end == cursor => Some(range.clone()),
                    _ => None,
                })
                .or_else(|| {
                    parsed.items().iter().find_map(|item| match item {
                        ParsedItem::Chord {
                            source_range: Some(range),
                            ..
                        } if range.start <= cursor && cursor < range.end => Some(range.clone()),
                        _ => None,
                    })
                });
        }
    }
    let units = sounding_units(line);
    let unit = units.get(unit_at(&units, cursor)?)?;
    unit.kind.is_sounding().then(|| unit.byte_range.clone())
}

#[cfg(test)]
mod tests;
