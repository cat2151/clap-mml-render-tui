//! アルペジエーター overlay の param pane で動かす、メイン画面と共有する行全体のルール（汚し・奏法）とアクセント。
//!
//! 値はルール表（[`GuitarArticulationScreen::rules`]）に置くので、閉じた後のメイン画面にも残る。
//! 値を変えても履歴には積まない。閉じたとき、開く前と違えばまとめて 1 件積む（[`GuitarArticulationScreen::take_unsaved_history`]）。

use crate::{AccentPattern, AutoPick, RowRule, RuleTable};

use super::arp::stepped;
use super::{GuitarArticulationAction, GuitarArticulationScreen, Take};

/// 汚し行の値（名前、汚し、汚し release）。並びは `h` / `l` で動く並び。
const HUMANIZE_CHOICES: [(&str, bool, bool); 4] = [
    ("汚しなし", false, false),
    ("汚し&汚しrelease", true, true),
    ("汚し", true, false),
    ("汚しrelease", false, true),
];

/// 奏法行の値（名前、エコノミーピッキング、自動ハンマリング）。並びは `h` / `l` で動く並び。
/// 自動ハンマリングとエコノミーピッキングは排他なので、組はこの 4 つで全部。
const PICKING_CHOICES: [(&str, bool, Option<AutoPick>); 4] = [
    ("ピッキング", false, None),
    ("エコ", true, None),
    ("オートプリング1", false, Some(AutoPick::Run)),
    ("オートプリング2", false, Some(AutoPick::Accent)),
];

/// アクセント行の値。並びは `h` / `l` で動く並び。
const ACCENT_CHOICES: [AccentPattern; 3] = [
    AccentPattern::Top,
    AccentPattern::Bottom,
    AccentPattern::Both,
];

fn humanize_index(rules: &RuleTable) -> usize {
    let on = (
        rules.is_row_on(RowRule::Humanize),
        rules.is_row_on(RowRule::HumanizeRelease),
    );
    HUMANIZE_CHOICES
        .iter()
        .position(|(_, humanize, release)| (*humanize, *release) == on)
        .unwrap_or_default()
}

fn picking_index(rules: &RuleTable) -> usize {
    let auto = rules
        .is_row_on(RowRule::AutoHammerPull)
        .then(|| rules.auto_pick());
    let economy = rules.is_row_on(RowRule::EconomyPicking);
    PICKING_CHOICES
        .iter()
        .position(|(_, choice_economy, choice_auto)| {
            (*choice_economy, *choice_auto) == (economy, auto)
        })
        .unwrap_or_default()
}

impl GuitarArticulationScreen {
    /// 汚し行に出す今の値。
    pub fn arp_humanize_label(&self) -> &'static str {
        HUMANIZE_CHOICES[humanize_index(&self.rules)].0
    }

    /// 奏法行に出す今の値。
    pub fn arp_picking_label(&self) -> &'static str {
        PICKING_CHOICES[picking_index(&self.rules)].0
    }

    /// 汚し行の値を `delta` だけ動かして鳴らす。端で止まり、変わらなければ何もしない。
    pub(super) fn step_arp_humanize(&mut self, delta: isize) -> GuitarArticulationAction {
        let index = humanize_index(&self.rules);
        let next = stepped(index, delta, 0..=HUMANIZE_CHOICES.len() - 1);
        if next == index {
            return GuitarArticulationAction::Continue;
        }
        let (_, humanize, release) = HUMANIZE_CHOICES[next];
        self.rules.set_row(RowRule::Humanize, humanize);
        self.rules.set_row(RowRule::HumanizeRelease, release);
        self.replay_arp_rules()
    }

    /// 奏法行の値を `delta` だけ動かして鳴らす。端で止まり、変わらなければ何もしない。
    pub(super) fn step_arp_picking(&mut self, delta: isize) -> GuitarArticulationAction {
        let index = picking_index(&self.rules);
        let next = stepped(index, delta, 0..=PICKING_CHOICES.len() - 1);
        if next == index {
            return GuitarArticulationAction::Continue;
        }
        let (_, economy, auto) = PICKING_CHOICES[next];
        self.rules.set_row(RowRule::EconomyPicking, economy);
        self.rules.set_auto_hammer_pull(auto);
        self.replay_arp_rules()
    }

    /// アクセント行の値を `delta` だけ動かして鳴らす。端で止まり、変わらなければ何もしない。
    pub(super) fn step_arp_accent(&mut self, delta: isize) -> GuitarArticulationAction {
        let pattern = self.rules.accent_pattern();
        let index = ACCENT_CHOICES
            .iter()
            .position(|choice| *choice == pattern)
            .unwrap_or_default();
        let next = stepped(index, delta, 0..=ACCENT_CHOICES.len() - 1);
        if next == index {
            return GuitarArticulationAction::Continue;
        }
        self.rules.set_accent_pattern(ACCENT_CHOICES[next]);
        self.replay_arp_rules()
    }

    /// ルール表を変えた後、Articulated を作り直して鳴らす。
    fn replay_arp_rules(&mut self) -> GuitarArticulationAction {
        self.rebuild_converted();
        self.play(Take::Converted)
    }

    /// overlay を閉じたとき行全体のルールが変わっていて、その履歴をまだ file へ書いていなければ
    /// true を返し、書いた扱いにする。
    pub fn take_unsaved_history(&mut self) -> bool {
        std::mem::take(&mut self.history_unsaved)
    }

    /// overlay を閉じるとき、開く前のルール表 `before` と違えば今の状態を履歴へ積み、file へ書く対象にする。
    pub(super) fn record_arp_rules(&mut self, before: &RuleTable) {
        if self.rules != *before {
            self.record_history();
            self.history_unsaved = true;
        }
    }
}

#[cfg(test)]
mod tests;
