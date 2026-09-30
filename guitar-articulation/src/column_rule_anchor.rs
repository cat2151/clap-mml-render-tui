//! 列ごとのルールを手で切り替えた直後の MML と列ルール。
//!
//! MML を確定し直したとき、この MML の列から新しい MML の列へ [`column_map`] で列ルールを付け替える元にする。
//! 更新するのは列ルールを手で切り替えたときだけで、MML を確定しても変えない。そのため、ある列を
//! 一時的に消して確定しても、足し直して確定すればその列のルールが戻る。

use serde::{Deserialize, Serialize};

use crate::column_map::column_map;
use crate::{notes_from_events, Note, RuleTable};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColumnRuleAnchor {
    #[serde(default)]
    pub mml: String,
    /// 列ごとのルールだけ。行全体のルールは持たない。
    #[serde(default)]
    pub rules: RuleTable,
}

impl ColumnRuleAnchor {
    /// `rules` の列ごとのルールだけを持つ。
    pub fn new(mml: &str, rules: &RuleTable) -> Self {
        ColumnRuleAnchor {
            mml: mml.to_string(),
            rules: RuleTable {
                on: rules.on.clone(),
                rows: Default::default(),
            },
        }
    }

    /// `notes` の列へ列ルールを付け替え、行全体のルールは `current` のものにした表。
    /// この MML を解釈できなければ、列ルールは空。
    pub(crate) fn rules_for(&self, notes: &[Note], current: &RuleTable) -> RuleTable {
        let mut rules = match cmrt_chord::timed_performance(&self.mml) {
            Ok(performance) => {
                let map = column_map(&notes_from_events(&performance.events), notes);
                self.rules.remap_columns(&map)
            }
            Err(_) => RuleTable::default(),
        };
        rules.rows = current.rows.clone();
        rules
    }
}

#[cfg(test)]
mod tests;
