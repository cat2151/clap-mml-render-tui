//! MML を書き換えた前後で、列番号を対応づける。
//!
//! 旧の列ごとに、note on の時刻がいちばん近い新の列を対応先にする。音高は見ない。
//! MML がまるごと別物になっても、同じ時間的位置の列へ付け替わる。

use std::collections::{BTreeMap, BTreeSet};

use crate::{Note, Rule, RuleTable};

/// 列ごとの note on 時刻。添字が列番号。
fn column_onsets(notes: &[Note]) -> Vec<f64> {
    let count = notes.iter().map(|note| note.column + 1).max().unwrap_or(0);
    let mut onsets = vec![f64::INFINITY; count];
    for note in notes {
        onsets[note.column] = onsets[note.column].min(note.on_seconds);
    }
    onsets
}

/// 旧の列番号 → 新の列番号。長さは旧の列数。等距離なら前の列、新が空なら `None`。
pub(crate) fn column_map(old: &[Note], new: &[Note]) -> Vec<Option<usize>> {
    let new = column_onsets(new);
    column_onsets(old)
        .into_iter()
        .map(|onset| {
            (0..new.len())
                .min_by(|&a, &b| (new[a] - onset).abs().total_cmp(&(new[b] - onset).abs()))
        })
        .collect()
}

impl RuleTable {
    /// 列ごとのルールを [`column_map`] の対応先の列へ付け替えた表。行ルールは持たない。
    /// 同じ列へ移る複数の列のルールは合わせ、排他のルールは前の列のものを残す。
    /// 対応先の無い列（範囲外・`None`）のルールは落とす。
    pub(crate) fn remap_columns(&self, map: &[Option<usize>]) -> RuleTable {
        let mut on: BTreeMap<usize, BTreeSet<Rule>> = BTreeMap::new();
        for (column, rules) in &self.on {
            if let Some(to) = map.get(*column).copied().flatten() {
                let merged: &mut BTreeSet<Rule> = on.entry(to).or_default();
                let has_exclusive = merged.iter().any(|rule| rule.is_exclusive());
                merged.extend(
                    rules
                        .iter()
                        .filter(|rule| !(has_exclusive && rule.is_exclusive())),
                );
            }
        }
        RuleTable {
            on,
            rows: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests;
