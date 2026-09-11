//! 「各 section の degrees の、どこからどこまでが 1 つの chord か」の写し。
//!
//! **切るのはこの crate ではない。** degrees は chord2mml-rs の書式をそのまま持つ
//! 文字列で、この画面は文字列を解釈しない（ADR 0020）。範囲を出すのは app 側の glue
//! （`cmrt_chord::chord_source_ranges()` ＝ chord 1 つを切り出して鳴らすときと同じ関数）で、
//! ここはその答えを写して持つだけ。[`ChordChartScreen::set_preview_sounding`] と同じ
//! 「自分では知り得ない答えを外から書き戻す」形。
//!
//! **chord の個数もここから来る**（`ranges.len()`）。数と範囲を別々の写しで持つと、
//! 片方だけ古くなったときに「4 個あると言っているのに範囲は 3 件」という食い違いが
//! 起きる。**写しは 1 つ**にして、数はその長さから derive する。
//!
//! 写しは **[`SectionId`] で引く**。行の index で持つと、写しが古いまま section を
//! 並べ替え / 削除したときに**別の section の答え**を返してしまう。id なら
//! 「知らない section」として `None` になり、[`ChordChartScreen::cursor_chord_count`]
//! は 1 を返す（＝ chord カーソルは 0 番から動けない）。

use std::ops::Range;

use super::ChordChartScreen;
use crate::SectionId;

/// 写しに答えが無い行の chord 数として扱う値。
///
/// 「読めない degrees の行は chord 1 個として扱う」という決めごとと同じ扱いにする。
/// 1 個なら chord カーソルは 0 番から動きようがないので、写しが届く前に `h` `l` を
/// 押しても行内では動かない（行をまたぐ繰り上がりは別の話）。
const CHORDS_WHEN_UNKNOWN: usize = 1;

impl ChordChartScreen {
    /// 各 section の chord の範囲を app 側から受け取る。**写しは丸ごと差し替える**
    /// （消えた section の答えが残らないように、差分更新にしない）。
    ///
    /// 範囲は **degrees の先頭からのバイト位置**（`chord_source_ranges` が返すそのまま）。
    /// 描画は表示幅で数えるので、桁へ直すのは `ui` 側の仕事。
    ///
    /// glue が呼ぶのは**曲が変わったときと画面へ入ったときだけ**。degrees が
    /// 変わらない限り範囲も変わらないので、キーごとに切り直す必要は無い。
    pub fn set_chord_ranges(
        &mut self,
        ranges: impl IntoIterator<Item = (SectionId, Vec<Range<usize>>)>,
    ) {
        self.chord_ranges = ranges.into_iter().collect();
    }

    /// 写しが答えを持っている section の数。書き戻される前は 0。
    ///
    /// 公開しているのは、「消した section の答えが写しに残っていないか」を
    /// app 側から確かめられるようにするため（写しの長さ = 曲の section 数）。
    pub fn chord_ranges_len(&self) -> usize {
        self.chord_ranges.len()
    }

    /// その section の chord 数。写しに無ければ `None`（この crate は数えない）。
    ///
    /// 読めない degrees は glue が 0 件を書き戻す。0 と `None` を区別して持つのは、
    /// 「まだ書き戻されていない」と「切ったら 0 件だった」を写しの上で潰さないため。
    pub fn chord_count(&self, id: SectionId) -> Option<usize> {
        self.chord_ranges.get(&id).map(|ranges| ranges.len())
    }

    /// カーソル行（右 pane なら参照先 section）の chord 数。**必ず 1 以上**。
    ///
    /// chord カーソルの範囲を決めるのはこの値。行が無い / 写しがまだ無い /
    /// 読めない degrees（0 件）は、どれも [`CHORDS_WHEN_UNKNOWN`] へ倒す。
    pub fn cursor_chord_count(&self) -> usize {
        self.preview_target()
            .and_then(|section| self.chord_count(section.id))
            .filter(|count| *count > 0)
            .unwrap_or(CHORDS_WHEN_UNKNOWN)
    }

    /// カーソル行の、いま指している chord の範囲（degrees 上の**バイト位置**）。
    ///
    /// 反転を描くための値。**無いことがある**（写しがまだ届いていない / 読めない
    /// degrees で 0 件 / 行が無い）。そのときは「反転を描かない」＝行がそのまま出る。
    pub fn cursor_chord_range(&self) -> Option<Range<usize>> {
        let id = self.preview_target()?.id;
        self.chord_ranges
            .get(&id)?
            .get(self.chord_cursor())
            .cloned()
    }
}

#[cfg(test)]
mod tests;
