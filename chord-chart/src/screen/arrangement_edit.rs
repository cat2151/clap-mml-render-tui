//! Arrangement pane（右）の編集: `1`..`9` 挿入 / `dd` 削除 / `Alt+↑` `Alt+↓` 移動。
//!
//! 曲を変えたら [`ChordChartAction::SongChanged`] を返す。**保存はここで呼ばない**
//! （呼び出し側の glue が受け取って書く）。何も変わらなかったときに `Continue` を返すのは、
//! 末尾での `Alt+↓` のような「押しても同じ」操作でファイルを書き直さないため。

use super::{ChordChartAction, ChordChartScreen, MoveDirection};
use crate::SectionId;

/// `1`..`9` で挿せる section の上限。左 pane の 10 行目以降は数字キーでは挿せない
/// （資料どおり。届かないだけなので `error` にも出さない）。
const MAX_INSERT_DIGIT: usize = 9;

impl ChordChartScreen {
    /// Arrangement pane（右）でだけ効くキー。
    pub(super) fn handle_arrangement_key(
        &mut self,
        code: crossterm::event::KeyCode,
    ) -> ChordChartAction {
        use crossterm::event::KeyCode;
        match code {
            KeyCode::Char(digit @ '1'..='9') => self.insert_section_by_digit(digit),
            _ => ChordChartAction::Continue,
        }
    }

    /// `1`..`9`: その番号の section をカーソルの**次**へ挿入する（空なら先頭へ）。
    ///
    /// 番号は**左 pane の行番号**（`song.sections` の index + 1）。右 pane の行番号ではない。
    /// 存在しない番号は何もしない（`error` にも出さない。押し間違いを毎回咎めても情報が無い）。
    fn insert_section_by_digit(&mut self, digit: char) -> ChordChartAction {
        let Some(index) = digit.to_digit(10).map(|value| value as usize) else {
            return ChordChartAction::Continue;
        };
        if index == 0 || index > MAX_INSERT_DIGIT {
            return ChordChartAction::Continue;
        }
        let Some(id) = self.song.sections.get(index - 1).map(|section| section.id) else {
            return ChordChartAction::Continue;
        };
        self.insert_at_cursor(id)
    }

    /// `dd`: カーソル行を削除する。section 自体は消さない（消すのは左 pane の `dd`）。
    pub(super) fn delete_arrangement_entry(&mut self) -> ChordChartAction {
        let Some(index) = self.selected_arrangement_index() else {
            return ChordChartAction::Continue;
        };
        self.song.arrangement.remove(index);
        // 末尾を消すとカーソルが 1 行はみ出す。丸めた値を書き戻しておかないと、
        // 次の `k` が「見た目は動かないのに index だけ減る」1 回になる。
        self.arrangement_cursor = self.clamped_arrangement_cursor();
        ChordChartAction::SongChanged
    }

    /// `Alt+↓` / `Alt+↑`: カーソル行を下 / 上へ 1 つ動かし、カーソルも一緒に動く。
    ///
    /// カーソルが追従しないと、押し続けたときに動かしている行が手元から離れていく。
    pub(super) fn move_arrangement_entry(&mut self, direction: MoveDirection) -> ChordChartAction {
        let Some(index) = self.selected_arrangement_index() else {
            return ChordChartAction::Continue;
        };
        let target = match direction {
            MoveDirection::Down => {
                Some(index + 1).filter(|next| *next < self.song.arrangement.len())
            }
            MoveDirection::Up => index.checked_sub(1),
        };
        // 端で押しても曲は変わらない＝保存もしない。
        let Some(target) = target else {
            return ChordChartAction::Continue;
        };
        self.song.arrangement.swap(index, target);
        self.arrangement_cursor = target;
        ChordChartAction::SongChanged
    }

    /// カーソルの次（空なら先頭）へ 1 行足し、カーソルを足した行へ移す。
    fn insert_at_cursor(&mut self, id: SectionId) -> ChordChartAction {
        let at = match self.selected_arrangement_index() {
            Some(index) => index + 1,
            None => 0,
        };
        self.song.arrangement.insert(at, id);
        self.arrangement_cursor = at;
        ChordChartAction::SongChanged
    }

    /// カーソルが指している arrangement の行。行が無いときは `None`。
    fn selected_arrangement_index(&self) -> Option<usize> {
        if self.song.arrangement.is_empty() {
            return None;
        }
        Some(self.clamped_arrangement_cursor())
    }
}

#[cfg(test)]
mod tests;
