//! 列ごとのルールを手で切り替えた直後の MML と列ルール。
//!
//! MML を確定し直したとき、この MML で鳴らした列から新しい列へ [`column_map`] で列ルールを付け替える元にする。
//! 更新するのは列ルールを手で切り替えたときだけで、MML を確定しても変えない。そのため、ある列を
//! 一時的に消して確定しても、足し直して確定すればその列のルールが戻る。

use serde::{Deserialize, Serialize};

use crate::column_map::column_map;
use crate::{notes_from_events, performance_events, Note, RuleTable};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ColumnRuleAnchor {
    #[serde(default)]
    pub mml: String,
    /// 列ごとのルールだけ。行全体のルールとパラメータは持たない。
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
                ..Default::default()
            },
        }
    }

    /// `notes` の列へ列ルールを付け替え、行全体のルール・パラメータ・アクセントと自動ハンマリングの選び方は `current` のものにした表。
    /// この MML を解釈できなければ、列ルールは空。
    pub(crate) fn rules_for(&self, notes: &[Note], current: &RuleTable) -> RuleTable {
        let mut rules = match performance_events(&self.mml, None) {
            Ok(events) => {
                let map = column_map(&notes_from_events(&events), notes);
                self.rules.remap_columns(&map)
            }
            Err(_) => RuleTable::default(),
        };
        rules.rows = current.rows.clone();
        rules.params = current.params.clone();
        rules.accent = current.accent;
        rules.auto_pick = current.auto_pick;
        rules
    }
}

#[cfg(test)]
mod tests;
