//! 列ごと・行全体のルールと、行全体のパラメータを持つ表。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{params, AccentPattern, AutoPick, RowRule, Rule};

/// 列番号 → その列で ON のルールと、行全体で ON のルールと、行全体のパラメータ（[`PARAMS`]）。
///
/// JSON では `{"columns":{"3":["hammer_pull"]},"rows":["economy_picking"],"params":{"22":0}}`。
/// `params` は既定と違う値だけで、無ければ省く。`accent`（[`AccentPattern`]）・`auto_pick`（[`AutoPick`]）も既定なら省く。
/// 排他な行ルール（[`RowRule::excluded`]）が両方 ON の JSON は、エコノミーピッキングだけ残して読む。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RuleTableFields")]
pub struct RuleTable {
    #[serde(rename = "columns")]
    pub(crate) on: BTreeMap<usize, BTreeSet<Rule>>,
    pub(crate) rows: BTreeSet<RowRule>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) params: BTreeMap<u8, u8>,
    #[serde(skip_serializing_if = "AccentPattern::is_default")]
    pub(crate) accent: AccentPattern,
    #[serde(skip_serializing_if = "AutoPick::is_default")]
    pub(crate) auto_pick: AutoPick,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleTableFields {
    #[serde(default)]
    columns: BTreeMap<usize, BTreeSet<Rule>>,
    #[serde(default)]
    rows: BTreeSet<RowRule>,
    #[serde(default)]
    params: BTreeMap<u8, u8>,
    #[serde(default)]
    accent: AccentPattern,
    #[serde(default)]
    auto_pick: AutoPick,
}

impl From<RuleTableFields> for RuleTable {
    fn from(fields: RuleTableFields) -> Self {
        let mut rows = fields.rows;
        if rows.contains(&RowRule::EconomyPicking) {
            rows.remove(&RowRule::AutoHammerPull);
        }
        let auto_pick = if rows.contains(&RowRule::AutoHammerPull) {
            fields.auto_pick
        } else {
            AutoPick::default()
        };
        RuleTable {
            on: fields.columns,
            rows,
            params: params::sanitized(fields.params),
            accent: fields.accent,
            auto_pick,
        }
    }
}

impl RuleTable {
    /// 1 行の JSON。ログから演奏を作り直すための綴り。
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("RuleTable is always serializable")
    }

    pub fn from_json(json: &str) -> Result<RuleTable, String> {
        serde_json::from_str(json).map_err(|err| format!("ルールの JSON を読めません: {err}"))
    }

    pub fn is_on(&self, column: usize, rule: Rule) -> bool {
        self.on
            .get(&column)
            .is_some_and(|rules| rules.contains(&rule))
    }

    /// ON/OFF を切り替える。ON にすると、同じ列で一緒に ON にできないルール（[`Rule::is_exclusive`] どうし、
    /// [`Rule::selects_release_shape`] どうし）は OFF になる。
    pub fn toggle(&mut self, column: usize, rule: Rule) {
        let rules = self.on.entry(column).or_default();
        if !rules.remove(&rule) {
            rules.retain(|other| !rule.conflicts_with(*other));
            rules.insert(rule);
        }
        if rules.is_empty() {
            self.on.remove(&column);
        }
    }

    /// 列ごとのルールが 1 つも ON でないか。行全体のルールは見ない。
    pub fn is_empty(&self) -> bool {
        self.on.is_empty()
    }

    /// そのルールが ON の列の数。
    pub fn column_count_of(&self, rule: Rule) -> usize {
        self.on
            .values()
            .filter(|rules| rules.contains(&rule))
            .count()
    }

    pub fn is_row_on(&self, rule: RowRule) -> bool {
        self.rows.contains(&rule)
    }

    /// エコノミーピッキングと汚しの強弱で、アクセントを付ける音の選び方。
    pub fn accent_pattern(&self) -> AccentPattern {
        self.accent
    }

    /// アクセントの選び方を次へ回す（[`AccentPattern::next`]）。
    pub fn cycle_accent_pattern(&mut self) {
        self.accent = self.accent.next();
    }

    /// アクセントの選び方を置く。
    pub fn set_accent_pattern(&mut self, pattern: AccentPattern) {
        self.accent = pattern;
    }

    /// 自動ハンマリングがピッキングする列の選び方。OFF の間は既定（[`AutoPick::Run`]）。
    pub fn auto_pick(&self) -> AutoPick {
        self.auto_pick
    }

    /// 自動ハンマリングを off → on1（[`AutoPick::Run`]）→ on2（[`AutoPick::Accent`]）→ off と回す。
    pub fn cycle_auto_hammer_pull(&mut self) {
        if !self.is_row_on(RowRule::AutoHammerPull) {
            self.toggle_row(RowRule::AutoHammerPull);
        } else if self.auto_pick == AutoPick::Run {
            self.auto_pick = AutoPick::Accent;
        } else {
            self.toggle_row(RowRule::AutoHammerPull);
        }
    }

    /// ON/OFF を切り替える。ON にすると、排他な相手（[`RowRule::excluded`]）は OFF になる。
    pub fn toggle_row(&mut self, rule: RowRule) {
        if !self.rows.remove(&rule) {
            if let Some(other) = rule.excluded() {
                self.rows.remove(&other);
            }
            self.rows.insert(rule);
        }
        if !self.is_row_on(RowRule::AutoHammerPull) {
            self.auto_pick = AutoPick::default();
        }
    }

    /// 行全体のルールを `on` にする。今と同じなら何もしない。排他と [`AutoPick`] の既定化は [`Self::toggle_row`] に従う。
    pub fn set_row(&mut self, rule: RowRule, on: bool) {
        if self.is_row_on(rule) != on {
            self.toggle_row(rule);
        }
    }

    /// 自動ハンマリングを置く。`None` で OFF、`Some` で ON にしてその選び方にする（エコノミーピッキングは OFF になる）。
    pub fn set_auto_hammer_pull(&mut self, pick: Option<AutoPick>) {
        self.set_row(RowRule::AutoHammerPull, pick.is_some());
        if let Some(pick) = pick {
            self.auto_pick = pick;
        }
    }

    /// 列ごとのルールだけを消した表（行ルールとパラメータは残す）。MML を確定し直すと列の位置が意味を失うので使う。
    pub fn without_column_rules(&self) -> RuleTable {
        RuleTable {
            on: BTreeMap::new(),
            ..self.clone()
        }
    }

    /// その列で H/P を効かせるか（列ごとの ON か、自動ハンマリングがレガートにする列）。
    /// `auto_picks` は [`auto_pick_columns`] の出力。
    pub fn hammer_pull_applies(&self, column: usize, auto_picks: &[bool]) -> bool {
        self.is_on(column, Rule::HammerPull)
            || (self.is_row_on(RowRule::AutoHammerPull) && !auto_picks[column])
    }
}
