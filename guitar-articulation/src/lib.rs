//! rawの MIDI イベント列を、METAL-GTX（sfz）のキースイッチ入りの列へ変換する。
//!
//! 変換は 1 ルール = 1 パスの純関数を順に積む（[`convert`] がその並び）。
//! KS 番号は METAL-GTX 固有なので、汎用の `cmrt-midi-filter` には置かない。
//! 変換を試す画面（[`GuitarArticulationScreen`] と [`ui`]）も同じ crate に置く。

mod hammer_pull;
mod keyswitch;
mod notes;
mod picking;
mod report;
mod screen;
mod strings;
pub mod ui;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub use cmrt_midi_filter::TimedMidiEvent;
pub use hammer_pull::apply_hammer_pull;
pub use keyswitch::{keyswitch_events, KEYSWITCH_RESET_SECONDS, KEYSWITCH_VELOCITY};
pub use notes::{notes_from_events, Note};
pub use picking::{articulate, Articulated, ECONOMY_UNACCENTED_VELOCITY_PERCENT};
pub use report::{compare, report};
pub use screen::{GuitarArticulationAction, GuitarArticulationScreen, Take, DEFAULT_MML};
pub use strings::{picks_string, strings_by_column, REACH_SEMITONES};

/// 演奏に使う音色（sforzando の `patches_dirs` からの相対）。KS 番号がこの音色に固有なので固定する。
pub const PATCH: &str = "sfz/UI_METAL-GTX/Programs/01-METAL-GTX Full.sfz";

/// 音ごとの奏法。METAL-GTX の KS（`sw_last` のラッチ式）1 つに対応する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Articulation {
    /// sfz の `sw_default`。ピッキングした音。
    SusDown,
    /// アップストロークでピッキングした音。
    SusUp,
    HammerOn,
    PullOff,
}

impl Articulation {
    /// この奏法を選ぶ KS の note number。
    pub fn keyswitch(self) -> u8 {
        match self {
            Articulation::SusDown => 17,
            Articulation::SusUp => 18,
            Articulation::HammerOn => 26,
            Articulation::PullOff => 25,
        }
    }

    /// METAL-GTX の KS 一覧（`METAL-GTX_KSMap.txt`）での名前。
    pub fn name(self) -> &'static str {
        match self {
            Articulation::SusDown => "Sus_Down",
            Articulation::SusUp => "Sus_Up",
            Articulation::HammerOn => "Hammer-On",
            Articulation::PullOff => "Pull-Off",
        }
    }

    /// KS の note number から奏法を引く。KS でない音高なら `None`。
    pub fn from_keyswitch(key: u8) -> Option<Self> {
        [
            Articulation::SusDown,
            Articulation::SusUp,
            Articulation::HammerOn,
            Articulation::PullOff,
        ]
        .into_iter()
        .find(|articulation| articulation.keyswitch() == key)
    }
}

/// 列ごとに ON/OFF するルール。次のルールはここへ variant を足す。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// 前の列から上行ならハンマリング、下行ならプリングオフ。
    HammerPull,
}

/// 行全体で ON/OFF するルール。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowRule {
    /// 各弦（[`strings_by_column`]）の最初の音だけピッキングし、残りを H/P にする。
    AutoHammerPull,
    /// イングヴェイ流のエコノミーピッキング（[`articulate`]）。
    EconomyPicking,
}

/// 列番号 → その列で ON のルールと、行全体で ON のルール。
///
/// JSON では `{"columns":{"3":["hammer_pull"]},"rows":["economy_picking"]}`。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleTable {
    #[serde(rename = "columns", default)]
    on: BTreeMap<usize, BTreeSet<Rule>>,
    #[serde(default)]
    rows: BTreeSet<RowRule>,
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

    pub fn toggle(&mut self, column: usize, rule: Rule) {
        let rules = self.on.entry(column).or_default();
        if !rules.remove(&rule) {
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

    pub fn is_row_on(&self, rule: RowRule) -> bool {
        self.rows.contains(&rule)
    }

    pub fn toggle_row(&mut self, rule: RowRule) {
        if !self.rows.remove(&rule) {
            self.rows.insert(rule);
        }
    }

    /// 列ごとのルールだけを消した表。MML を確定し直すと列の位置が意味を失うので使う。
    pub fn without_column_rules(&self) -> RuleTable {
        RuleTable {
            on: BTreeMap::new(),
            rows: self.rows.clone(),
        }
    }

    /// その列で H/P を効かせるか（列ごとの ON か、自動ハンマリングがレガートにする列）。
    /// `strings` は [`strings_by_column`] の出力。
    pub fn hammer_pull_applies(&self, column: usize, strings: &[usize]) -> bool {
        self.is_on(column, Rule::HammerPull)
            || (self.is_row_on(RowRule::AutoHammerPull) && !picks_string(strings, column))
    }
}

/// rawの列に、ルール表から作った KS を足した演奏用の列を返す。
///
/// 元のイベントは時刻を変えずに残し、note on の velocity だけ [`articulate`] の値へ差し替える。
/// KS を元の列より前に積んでから stable sort するので、同時刻では KS の note on が
/// 演奏音の note on より前に来る（後だとその音に KS が効かない）。
pub fn convert(events: &[TimedMidiEvent], rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(events);
    let articulated = articulate(&notes, rules);
    let articulations: Vec<Articulation> = articulated.iter().map(|a| a.articulation).collect();
    let mut out = keyswitch_events(&notes, &articulations);
    out.extend(events.iter().map(|event| {
        let mut event = *event;
        if let Some(velocity) = picking::velocity_for(&event, &notes, &articulated) {
            event.message[2] = velocity;
        }
        event
    }));
    cmrt_midi_filter::sort_for_playback(&mut out);
    out
}
