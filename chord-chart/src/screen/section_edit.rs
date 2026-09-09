//! Sections pane（左）の編集: `g` 抽選追加 / `r` 引き直し / `dd` 削除 /
//! `Alt+↑` `Alt+↓` 並べ替え。
//!
//! 曲を変えたら [`ChordChartAction::SongChanged`] を返す。**保存はここで呼ばない**
//! （呼び出し側の glue が受け取って書く）。何も変わらなかったときに `Continue` を返すのは、
//! 端に当たった `Alt+↑` のような「押しても同じ」操作でファイルを書き直さないため。

use super::{ChordChartAction, ChordChartScreen, MoveDirection};
use crate::catalog::pick_progression;

/// 自動採番に使う名前の見出し。使い切ったら `A2` … と数字を足す。
const NAME_LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

impl ChordChartScreen {
    /// `g`: カタログから抽選して section を足し、カーソルをその行へ移す。
    ///
    /// カーソルを移すのは、直後の `r` / `i` / `n` が**足したばかりのもの**に
    /// 効いてほしいから（足したあと自分で探して降りるのは無駄な操作）。
    pub(super) fn add_section_from_catalog(&mut self) -> ChordChartAction {
        let degrees = match pick_progression(self.chord_progression_source()) {
            Ok(degrees) => degrees,
            Err(reason) => {
                self.error = Some(reason);
                return ChordChartAction::Continue;
            }
        };
        let name = next_section_name(&self.song);
        self.song.push_section(name, degrees);
        self.section_cursor = self.song.sections.len().saturating_sub(1);
        ChordChartAction::SongChanged
    }

    /// 保存ファイルが無かったときの最初の 1 つ。`g` と同じ抽選をして、
    /// **arrangement にも 1 行置く**（section だけだと右 pane が空のままで、
    /// 何から始めればいいのか分からない画面になる）。
    ///
    /// 進行をここでハードコードしないのが要点。曲の中身はカタログから引いたものだけ。
    pub(super) fn generate_initial_section(&mut self) -> ChordChartAction {
        if self.add_section_from_catalog() == ChordChartAction::Continue {
            // 抽選できなかった（カタログが無い）。理由は `error` に入っている。
            return ChordChartAction::Continue;
        }
        let Some(added) = self.song.sections.last().map(|section| section.id) else {
            return ChordChartAction::Continue;
        };
        self.song.arrangement.push(added);
        ChordChartAction::SongChanged
    }

    /// `r`: カーソル section の進行だけを引き直す（名前は保つ）。
    ///
    /// 名前を保つのは、arrangement が id を持っている＝並びは変わらないから。
    /// 「B の中身だけ差し替える」が曲の形を崩さずに試せる操作になる。
    pub(super) fn reroll_selected_section(&mut self) -> ChordChartAction {
        let Some(index) = self.selected_section_index() else {
            return ChordChartAction::Continue;
        };
        let degrees = match pick_progression(self.chord_progression_source()) {
            Ok(degrees) => degrees,
            Err(reason) => {
                self.error = Some(reason);
                return ChordChartAction::Continue;
            }
        };
        if self.song.sections[index].degrees == degrees {
            return ChordChartAction::Continue;
        }
        self.song.sections[index].degrees = degrees;
        ChordChartAction::SongChanged
    }

    /// `dd`: カーソル section を削除し、arrangement 上の参照も全部消す。
    ///
    /// 参照を残すと「引けない id」の行が右 pane に増える。行を残す価値は無い
    /// （消したのは素材そのもので、並びの穴ではない）。
    pub(super) fn delete_selected_section(&mut self) -> ChordChartAction {
        let Some(index) = self.selected_section_index() else {
            return ChordChartAction::Continue;
        };
        let removed = self.song.sections.remove(index).id;
        self.song.arrangement.retain(|id| *id != removed);
        // 末尾を消すとカーソルが 1 行はみ出す。丸めた値を書き戻しておかないと、
        // 次の `k` が「見た目は動かないのに index だけ減る」1 回になる。
        self.section_cursor = self.clamped_section_cursor();
        self.arrangement_cursor = self.clamped_arrangement_cursor();
        ChordChartAction::SongChanged
    }

    /// `Alt+↑` / `Alt+↓`: カーソル section を上 / 下へ 1 つ動かし、カーソルも一緒に動く。
    ///
    /// 動かすのは **`sections` の並びだけ**。arrangement は [`crate::SectionId`] を
    /// 持っているので曲の並びは変わらず、`1`..`9` の指す先だけが追従する。
    pub(super) fn move_section(&mut self, direction: MoveDirection) -> ChordChartAction {
        let Some(index) = self.selected_section_index() else {
            return ChordChartAction::Continue;
        };
        let target = match direction {
            MoveDirection::Down => Some(index + 1).filter(|next| *next < self.song.sections.len()),
            MoveDirection::Up => index.checked_sub(1),
        };
        // 端で押しても曲は変わらない＝保存もしない。
        let Some(target) = target else {
            return ChordChartAction::Continue;
        };
        self.song.sections.swap(index, target);
        self.section_cursor = target;
        ChordChartAction::SongChanged
    }

    /// カーソルが指している section の index。行が無いときは `None`。
    pub(super) fn selected_section_index(&self) -> Option<usize> {
        if self.song.sections.is_empty() {
            return None;
        }
        Some(self.clamped_section_cursor())
    }
}

/// まだ使われていない名前を 1 つ作る。`A`..`Z` → `A2`..`Z2` → `A3` …
///
/// 「今ある section の数 + 1 文字目」ではなく**未使用のものを探す**のは、
/// `B` を消してから足したときに `C` が 2 つできないようにするため。
fn next_section_name(song: &crate::Song) -> String {
    let used: Vec<&str> = song
        .sections
        .iter()
        .map(|section| section.name.as_str())
        .collect();
    for round in 1u32.. {
        for letter in NAME_LETTERS.chars() {
            let candidate = if round == 1 {
                letter.to_string()
            } else {
                format!("{letter}{round}")
            };
            if !used.iter().any(|name| *name == candidate) {
                return candidate;
            }
        }
    }
    unreachable!("the candidate space is unbounded")
}

#[cfg(test)]
mod tests;
