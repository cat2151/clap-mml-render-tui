//! 「各 section の degrees の、どこからどこまでが 1 つの chord か」の写し。
//!
//! 切るのはこの crate ではない（`docs/adr/0020`）。答えは app 側の glue
//! （`cmrt_chord::chord_source_ranges()`）が書き戻し、chord の個数も `ranges.len()` から
//! derive する。数を別の写しで持つと、片方だけ古くなったときに食い違う。
//!
//! 写しは [`SectionId`] で引く。行 index で持つと、写しが古いまま section を並べ替え / 削除
//! したときに別の section の答えを返す。id なら `None` になり、chord カーソルは 0 番に留まる。

use std::ops::Range;

use super::ChordChartScreen;
use crate::SectionId;

/// 写しに答えが無い行の chord 数。「読めない degrees の行は chord 1 個」と同じ扱いで、
/// 1 個なら写しが届く前に `h` `l` を押しても行内では動かない。
const CHORDS_WHEN_UNKNOWN: usize = 1;

impl ChordChartScreen {
    /// 各 section の chord の範囲を app 側から受け取る。写しは丸ごと差し替える
    /// （差分更新にすると、消えた section の答えが残る）。
    ///
    /// 範囲は degrees の先頭からのバイト位置。描画は表示幅で数えるので、桁へ直すのは `ui` 側。
    pub fn set_chord_ranges(
        &mut self,
        ranges: impl IntoIterator<Item = (SectionId, Vec<Range<usize>>)>,
    ) {
        self.chord_ranges = ranges.into_iter().collect();
    }

    /// 写しが答えを持っている section の数（app 側が「消した section の答えが残っていないか」を見る）。
    pub fn chord_ranges_len(&self) -> usize {
        self.chord_ranges.len()
    }

    /// その section の chord 数。写しに無ければ `None`。
    /// 0（読めない degrees）と `None`（まだ書き戻されていない）は区別して持つ。
    pub fn chord_count(&self, id: SectionId) -> Option<usize> {
        self.chord_ranges.get(&id).map(|ranges| ranges.len())
    }

    /// カーソル行（右 pane なら参照先 section）の chord 数。必ず 1 以上で、chord カーソルの範囲を決める。
    /// 行が無い / 写しがまだ無い / 0 件は、どれも [`CHORDS_WHEN_UNKNOWN`] へ倒す。
    pub fn cursor_chord_count(&self) -> usize {
        self.preview_target()
            .and_then(|section| self.chord_count(section.id))
            .filter(|count| *count > 0)
            .unwrap_or(CHORDS_WHEN_UNKNOWN)
    }

    /// カーソル行の、いま指している chord の範囲（degrees 上のバイト位置）。反転を描くための値。
    /// 無いこと（写し未着 / 0 件 / 行が無い）があり、そのときは反転を描かず行がそのまま出る。
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
