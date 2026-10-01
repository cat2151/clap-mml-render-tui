//! 奏法リスト overlay（`t`）。列ルールを matrix と同じ 5 段（[`RULE_LANES`]）に分けて並べる。
//! j/k で段を選び、h/l で段の中の項目を左右へ動かして、動かした先をすぐカーソル列で ON にして鳴らす
//! （左端の「なし」は段のルールを全部 OFF）。overlay の文字キーでそのルールを ON/OFF する。
//! space で試聴、Enter / Esc で閉じる。`/` で項目を絞り込む（表示名と別名に、画面横断の絞り込み条件を当てる）。
//!
//! 段の中で選んでいる項目は、カーソル列で ON のもの（OFF なら「なし」）で、別の状態として持たない。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::TextArea;

use cmrt_tui_core::text_filter::{compile_condition, matches_any_field};
use cmrt_tui_core::text_input::{
    apply_key_event_to_textarea, new_single_line_textarea, textarea_value,
};

use crate::ui::{RuleLane, RuleRow, RULE_LANES, RULE_ROWS};
use crate::{Rule, Take};

use super::input::is_commit_key;
use super::{GuitarArticulationAction, GuitarArticulationScreen};

/// 奏法リストに見えている段 1 つと、その中で絞り込みに当たった項目（[`RULE_ROWS`] の順）。
pub(crate) struct ListedLane {
    pub lane: RuleLane,
    pub rules: Vec<&'static RuleRow>,
}

/// 奏法リスト overlay の状態。閉じたら捨てるので、絞り込みは開くたびに空から始まる。
#[derive(Default)]
pub(super) struct RuleList {
    /// 選んでいる段。絞り込みで段が 1 つも見えないときだけ `None`。
    lane: Option<RuleLane>,
    /// 入力欄の文字列（不正な正規表現のこともある）。
    query: String,
    /// 項目を絞っている条件。`query` が不正な間は、直前の有効な条件のまま。
    applied_query: String,
    /// `/` を押してから Enter / Esc で抜けるまでの入力状態。
    input: Option<FilterInput>,
}

/// Esc は「絞り込みの解除」ではなく「`/` を押す前へ戻す」なので、戻し先を控える。
struct FilterInput {
    textarea: TextArea<'static>,
    query_before: String,
    applied_query_before: String,
    lane_before: Option<RuleLane>,
}

impl RuleList {
    /// 絞り込み後に見えている段。当たりが 0 個の段は含めない。
    fn listed_lanes(&self) -> Vec<ListedLane> {
        let condition = compile_condition(&self.applied_query).ok();
        RULE_LANES
            .iter()
            .filter_map(|&lane| {
                let rules: Vec<&'static RuleRow> = lane
                    .rule_rows()
                    .filter(|rule_row| match &condition {
                        Some(condition) => {
                            let mut fields = vec![rule_row.name];
                            fields.extend_from_slice(rule_row.aliases());
                            matches_any_field(condition, &fields)
                        }
                        None => true,
                    })
                    .collect();
                (!rules.is_empty()).then_some(ListedLane { lane, rules })
            })
            .collect()
    }

    fn is_listed(&self, lane: RuleLane) -> bool {
        self.listed_lanes().iter().any(|listed| listed.lane == lane)
    }

    /// 入力欄の文字列を差し替える。正しい条件なら項目を絞り直し、選んでいる段が消えたら見えている先頭の段へ。
    fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        if compile_condition(query).is_err() {
            return;
        }
        self.applied_query = query.to_string();
        if !self.lane.is_some_and(|lane| self.is_listed(lane)) {
            self.lane = self.listed_lanes().first().map(|listed| listed.lane);
        }
    }

    /// 選んでいる段を見えている段の中で `delta` だけ動かす（端で止まる）。
    fn move_lane(&mut self, delta: isize) {
        let lanes: Vec<RuleLane> = self
            .listed_lanes()
            .iter()
            .map(|listed| listed.lane)
            .collect();
        let Some(index) = self
            .lane
            .and_then(|lane| lanes.iter().position(|&listed| listed == lane))
        else {
            return;
        };
        let next = index
            .saturating_add_signed(delta)
            .min(lanes.len().saturating_sub(1));
        self.lane = Some(lanes[next]);
    }
}

impl GuitarArticulationScreen {
    pub fn rule_list_open(&self) -> bool {
        self.rule_list.is_some()
    }

    /// 奏法リストで選んでいる段。閉じているか、絞り込みで段が 1 つも見えなければ `None`。
    pub(crate) fn rule_list_lane(&self) -> Option<RuleLane> {
        self.rule_list.as_ref()?.lane
    }

    /// 奏法リストに見えている段。閉じていれば空。
    pub(crate) fn rule_list_lanes(&self) -> Vec<ListedLane> {
        self.rule_list
            .as_ref()
            .map(RuleList::listed_lanes)
            .unwrap_or_default()
    }

    /// 段の中で選ばれている項目。見えている項目のうちカーソル列で ON のもので、無ければ「なし」（`None`）。
    pub(crate) fn rule_list_choice(&self, listed: &ListedLane) -> Option<Rule> {
        listed
            .rules
            .iter()
            .map(|rule_row| rule_row.rule)
            .find(|&rule| self.rules.is_on(self.cursor, rule))
    }

    /// 奏法リストの絞り込みの入力欄の文字列。閉じていれば空。
    pub fn rule_list_query(&self) -> &str {
        self.rule_list
            .as_ref()
            .map_or("", |rule_list| rule_list.query.as_str())
    }

    /// 奏法リストの絞り込みを入力中か。入力中はすべてのキーが入力欄へ入る。
    pub fn rule_list_filter_input_active(&self) -> bool {
        self.rule_list_filter_textarea().is_some()
    }

    pub(crate) fn rule_list_filter_textarea(&self) -> Option<&TextArea<'static>> {
        self.rule_list
            .as_ref()?
            .input
            .as_ref()
            .map(|input| &input.textarea)
    }

    pub(super) fn open_rule_list(&mut self) -> GuitarArticulationAction {
        if self.column_count() == 0 {
            self.error = Some("i で MML を入力してください".to_string());
        } else {
            self.rule_list = Some(RuleList {
                lane: Some(RULE_LANES[0]),
                ..RuleList::default()
            });
        }
        GuitarArticulationAction::Continue
    }

    pub(super) fn handle_rule_list_key(&mut self, key: KeyEvent) -> GuitarArticulationAction {
        let Some(rule_list) = self.rule_list.as_mut() else {
            return GuitarArticulationAction::Continue;
        };
        if rule_list.input.is_some() {
            handle_filter_input_key(rule_list, key);
            return GuitarArticulationAction::Continue;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Enter => self.rule_list = None,
            KeyCode::Char('j') | KeyCode::Down => rule_list.move_lane(1),
            KeyCode::Char('k') | KeyCode::Up => rule_list.move_lane(-1),
            KeyCode::Char('/') => {
                let mut textarea = new_single_line_textarea(&rule_list.query);
                textarea.move_cursor(ratatui_textarea::CursorMove::End);
                rule_list.input = Some(FilterInput {
                    textarea,
                    query_before: rule_list.query.clone(),
                    applied_query_before: rule_list.applied_query.clone(),
                    lane_before: rule_list.lane,
                });
            }
            KeyCode::Char('l') | KeyCode::Right => return self.move_rule_list_choice(1),
            KeyCode::Char('h') | KeyCode::Left => return self.move_rule_list_choice(-1),
            KeyCode::Char(' ') => return self.play(Take::Converted),
            KeyCode::Char(ch) => {
                if let Some(rule_row) = RULE_ROWS.iter().find(|row| row.overlay_key == ch) {
                    let lane = RuleLane::of(rule_row.rule);
                    if rule_list.is_listed(lane) {
                        rule_list.lane = Some(lane);
                    }
                    return self.toggle_rule(rule_row.rule);
                }
            }
            _ => {}
        }
        GuitarArticulationAction::Continue
    }

    /// 選んでいる段の中で、選ばれている項目を `delta` だけ動かした先をカーソル列で ON にする
    /// （「なし」なら段のルールを OFF にする）。端で止まり、動かなければ鳴らさない。
    fn move_rule_list_choice(&mut self, delta: isize) -> GuitarArticulationAction {
        let lane = self.rule_list_lane();
        let Some(listed) = self
            .rule_list_lanes()
            .into_iter()
            .find(|listed| Some(listed.lane) == lane)
        else {
            return GuitarArticulationAction::Continue;
        };
        let current = self.rule_list_choice(&listed);
        let choices: Vec<Option<Rule>> = std::iter::once(None)
            .chain(listed.rules.iter().map(|rule_row| Some(rule_row.rule)))
            .collect();
        let index = choices
            .iter()
            .position(|&choice| choice == current)
            .unwrap_or(0);
        let next = index.saturating_add_signed(delta).min(choices.len() - 1);
        if next == index {
            return GuitarArticulationAction::Continue;
        }
        // ON にすると同じ段の他のルールは排他で OFF になる。「なし」へは、ON のルールを OFF にする。
        match choices[next].or(current) {
            Some(rule) => self.toggle_rule(rule),
            None => GuitarArticulationAction::Continue,
        }
    }
}

/// 絞り込み入力中のキー。自前で拾うのは Enter（確定）と Esc（巻き戻し）だけで、残りは入力欄へ渡す。
fn handle_filter_input_key(rule_list: &mut RuleList, key: KeyEvent) {
    if is_commit_key(key) {
        rule_list.input = None;
        return;
    }
    if key.code == KeyCode::Esc {
        if let Some(input) = rule_list.input.take() {
            rule_list.query = input.query_before;
            rule_list.applied_query = input.applied_query_before;
            rule_list.lane = input.lane_before;
        }
        return;
    }
    let Some(input) = rule_list.input.as_mut() else {
        return;
    };
    if apply_key_event_to_textarea(&mut input.textarea, key) {
        let query = textarea_value(&input.textarea);
        rule_list.set_query(&query);
    }
}
