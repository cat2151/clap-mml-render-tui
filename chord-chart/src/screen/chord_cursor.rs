//! 行内の chord カーソル（`h` `l` `←` `→`）。
//!
//! 画面が持つのは **index だけ**。「行内に chord が何個あるか」は
//! [`ChordChartScreen::cursor_chord_count`]（app が書き戻した写し）から引き、
//! 「何番目がどこからどこまでか」は app 側の glue が切る。この crate は degrees を
//! 1 文字も読まない（ADR 0020）。
//!
//! **鳴らすのは chord 1 つ**（要求の `chord_index` が埋まる）。行そのものを移る
//! `j` `k` `PgUp` `PgDn` は今までどおり**行全体**を鳴らし、chord カーソルは
//! 先頭へ戻る（行送りと chord 送りで鳴るものが変わるほうが、いま何を聴いているのかが
//! 分かる。組み立てのコストは chord 1 つと行全体で 59 µs しか違わない）。

use super::{ChordChartScreen, Pane};

/// `h` `l` `←` `→` の向き。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChordStep {
    /// `l` `→`: 次の chord。行末なら**次の行の先頭 chord**へ繰り上がる。
    Next,
    /// `h` `←`: 前の chord。行頭なら**前の行の末尾 chord**へ繰り上がる。
    Previous,
}

impl ChordChartScreen {
    /// いま行内の何番目の chord を指しているか（**0 始まり**）。
    ///
    /// 読むときは必ずここを通すこと。生のフィールドは、degrees を打ち替えて chord が
    /// 減ったあと（`i` の確定では preview 要求が立たないので、その場では丸め直す
    /// 機会が無い）に範囲外のまま残りうる。[`ChordChartScreen::cursor_chord_count`]
    /// が必ず 1 以上を返すので、戻り値は行に必ず存在する番号になる。
    pub fn chord_cursor(&self) -> usize {
        self.chord_cursor
            .min(self.cursor_chord_count().saturating_sub(1))
    }

    /// chord カーソルを先頭へ戻す。行が変わったとき（`j` `k` `PgUp` `PgDn`
    /// `Alt+↑` `Alt+↓`・pane 移動）に呼ぶ。
    ///
    /// 移った先の行が何 chord あるかは行ごとに違うので、番号を持ち越すと
    /// 「4 chord の行で 3 番目を聴いていて、2 chord の行へ降りたら末尾になる」
    /// という、押していないのに位置が変わる動きになる。
    pub(super) fn reset_chord_cursor(&mut self) {
        self.chord_cursor = 0;
    }

    /// `Tab`: pane を入れ替える（2 pane なのでトグル）。
    ///
    /// `Shift+Tab` は割り当てない（行き先が 1 つしか無いので逆回しに意味が無い）。
    pub(super) fn toggle_pane(&mut self) {
        let next = match self.focus {
            Pane::Sections => Pane::Arrangement,
            Pane::Arrangement => Pane::Sections,
        };
        self.focus_pane(next);
    }

    /// `h` `l` `←` `→`: 行内の chord カーソルを 1 つ動かす。
    ///
    /// 行の端では**隣の行へ繰り上がる**（末尾で `l` なら次の行の先頭、先頭で `h` なら
    /// 前の行の末尾）。繰り上がりは **pane をまたがない**ので、最終行の末尾で `l` /
    /// 先頭行の先頭で `h` はそこで止まり、**要求も立たない**
    /// （端で連打しても同じ音が鳴り直さない、という行移動と同じ規則）。
    pub(super) fn move_chord_cursor(&mut self, step: ChordStep) {
        let before = self.cursor_position();
        match step {
            ChordStep::Next => {
                let next = self.chord_cursor() + 1;
                if next < self.cursor_chord_count() {
                    self.chord_cursor = next;
                } else if self.step_row(RowStep::Next) {
                    self.chord_cursor = 0;
                }
            }
            ChordStep::Previous => match self.chord_cursor().checked_sub(1) {
                Some(previous) => self.chord_cursor = previous,
                None => {
                    if self.step_row(RowStep::Previous) {
                        // 移った**あと**に数えること。末尾の番号は移った先の行のもの。
                        self.chord_cursor = self.cursor_chord_count() - 1;
                    }
                }
            },
        }
        if self.cursor_position() != before {
            self.request_chord_preview();
        }
    }

    /// 繰り上がりのために、フォーカスしている pane の行を 1 つ動かす。
    /// 動けたら `true`（端なら何もせず `false`）。
    fn step_row(&mut self, step: RowStep) -> bool {
        let (current, len) = match self.focus {
            Pane::Sections => (self.clamped_section_cursor(), self.song.sections.len()),
            Pane::Arrangement => (
                self.clamped_arrangement_cursor(),
                self.song.arrangement.len(),
            ),
        };
        let target = match step {
            RowStep::Next => Some(current + 1).filter(|next| *next < len),
            RowStep::Previous => current.checked_sub(1).filter(|_| len > 0),
        };
        let Some(target) = target else {
            return false;
        };
        match self.focus {
            Pane::Sections => self.section_cursor = target,
            Pane::Arrangement => self.arrangement_cursor = target,
        }
        true
    }
}

/// 繰り上がりで行をどちらへ動かすか。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RowStep {
    Next,
    Previous,
}

#[cfg(test)]
mod tests;
