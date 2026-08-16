//! カーソル位置を、そこにある「発音単位」の範囲へ写す。
//!
//! 発音単位の切り出しそのものは本家 mmlabc-to-smf が CST から返す
//! （`sounding_units`）。ここが持つのは MML 経路と chord2mml 経路のどちらを
//! 使うかの選択だけで、MML の文法はひとつも持たない。
//!
//! コード表記の行を別扱いにするのは、chord2mml が入力を書き換えてから解釈する
//! ため、変換後の位置を元テキストへ戻せないから。そちらは従来どおり
//! 「行頭からカーソルまで」を 1 つの単位として扱う。

use std::ops::Range;

use mmlabc_to_smf::sounding_units::{sounding_units, unit_at};

use crate::timed::resolve_chord_or_mml;

/// カーソルのある発音単位の範囲。鳴らすものが無ければ `None`。
///
/// `cursor` は行頭からのバイト位置。返る範囲は
///
/// - MML の行なら、その発音単位ひとつ。和音は両方のクォートを含めて 1 つ。
///   休符やコマンド（`o5` `l8` `<` など）の上では `None`。
/// - コード表記の行なら `cursor..cursor`。
///
/// 範囲の終端まで解釈すれば、その単位の音がそのまま鳴る。カーソルの手前で
/// 切らないので、`c4` の途中にカーソルがあっても 4 分音符として鳴る。
pub fn cursor_sounding_unit(line: &str, cursor: usize) -> Option<Range<usize>> {
    if resolve_chord_or_mml(line).from_chord {
        return (cursor > 0).then_some(cursor..cursor);
    }
    let units = sounding_units(line);
    let unit = units.get(unit_at(&units, cursor)?)?;
    unit.kind.is_sounding().then(|| unit.byte_range.clone())
}

#[cfg(test)]
mod tests;
