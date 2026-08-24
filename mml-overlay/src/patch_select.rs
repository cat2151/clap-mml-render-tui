//! MML オーバーレイから開く音色選択。
//!
//! 左から「Grid Sequencer 上の大分類」「正規表現プリセット」「音色」の 3 pane。
//! 手入力した正規表現とプリセットを AND で組み合わせる。選択そのものはここに閉じ、
//! 音を鳴らす/JSON へ永続化する処理は [`crate::state`] と host app に任せる。

mod filter;
mod prepared;
mod presets;

use std::{collections::BTreeMap, sync::Arc};

use cmrt_patches::{PatchRoleIndex, PatchRoleInput};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use cmrt_tui_core::{patch_load::PatchLoadMeasurement, text_input};

use crate::PatchCatalogEntry;

use filter::{filter_candidates, is_valid_condition};
use prepared::PreparedPresets;
use presets::{normalize_user_presets, patterns_for_role, FilterGroup, FilterPreset};

const PAGE_STEP: isize = 10;

/// 左右キーでどの pane のカーソルを上下移動するか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatchSelectFocus {
    Groups,
    Presets,
    Patches,
}

/// 音色選択が呼び出し側へ求める処理。
pub(crate) enum PatchSelectAction {
    /// 表示が変わっただけ。
    Continue,
    /// この音色を試聴する。
    Preview(String),
    /// この音色で確定して閉じる。
    Confirm(String),
    /// ユーザー追加プリセットを JSON へ保存する。
    SaveUserPresets {
        presets: Vec<(String, String)>,
        preview: Option<String>,
    },
    /// 取り消して閉じる。開いたときの音色へ戻す。
    Cancel,
}

pub(crate) struct PatchSelect<'a> {
    all: Vec<PatchCatalogEntry>,
    filtered: Arc<[usize]>,
    cursor: usize,
    query: TextArea<'a>,
    filter_error: Option<String>,
    user_presets: Vec<(String, String)>,
    role_index: PatchRoleIndex,
    prepared_presets: PreparedPresets,
    group_cursor: usize,
    preset_cursor: usize,
    focus: PatchSelectFocus,
    /// 開いたときの音色。取り消しで戻す先。
    original: Option<String>,
    /// 直近に試聴した音色。同じ音色を続けて読み込ませないために持つ。
    previewed: Option<String>,
    /// 設定不足でカタログから外れたプラグインの案内。枠の下へそのまま出す。
    catalog_notes: Vec<String>,
    load_measurements: BTreeMap<String, PatchLoadMeasurement>,
}

impl<'a> PatchSelect<'a> {
    /// 音色が 1 つも無ければ開かない（`None` を返す）。
    pub(crate) fn open(
        mut all: Vec<PatchCatalogEntry>,
        current: Option<&str>,
        user_presets: Vec<(String, String)>,
        mut role_index: PatchRoleIndex,
        catalog_notes: Vec<String>,
        load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    ) -> Option<Self> {
        if all.is_empty() {
            return None;
        }
        all.sort_by(|left, right| left.selector_sort_key().cmp(&right.selector_sort_key()));
        let user_presets = normalize_user_presets(user_presets)
            .into_iter()
            .filter(|(_, pattern)| is_valid_condition(pattern))
            .collect::<Vec<_>>();
        if role_index.is_empty() {
            role_index = build_role_index(&all, &user_presets);
        }
        let prepared_presets = PreparedPresets::build(&all, &user_presets, &role_index)
            .expect("validated preset regular expressions must compile");
        let filtered = Arc::clone(&prepared_presets.for_role(0)[0].matches);
        let cursor = current
            .and_then(|current| {
                filtered
                    .iter()
                    .position(|index| all[*index].display() == current)
            })
            .unwrap_or(0);
        Some(Self {
            all,
            filtered,
            cursor,
            query: text_input::new_single_line_textarea(""),
            filter_error: None,
            user_presets,
            role_index,
            prepared_presets,
            group_cursor: 0,
            preset_cursor: 0,
            focus: PatchSelectFocus::Patches,
            original: current.map(str::to_string),
            previewed: current.map(str::to_string),
            catalog_notes,
            load_measurements,
        })
    }

    pub(crate) fn query_textarea(&self) -> &TextArea<'a> {
        &self.query
    }

    pub(crate) fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    pub(crate) fn groups(&self) -> &[FilterGroup] {
        &FilterGroup::ALL
    }

    pub(crate) fn group_cursor(&self) -> usize {
        self.group_cursor
    }

    pub(crate) fn presets(&self) -> &[FilterPreset] {
        self.prepared_presets.for_role(self.group_cursor)
    }

    pub(crate) fn preset_cursor(&self) -> usize {
        self.preset_cursor
    }

    pub(crate) fn focus(&self) -> PatchSelectFocus {
        self.focus
    }

    pub(crate) fn filtered(&self) -> impl ExactSizeIterator<Item = &PatchCatalogEntry> {
        self.filtered.iter().map(|index| &self.all[*index])
    }

    pub(crate) fn filtered_len(&self) -> usize {
        self.filtered.len()
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn total(&self) -> usize {
        self.all.len()
    }

    /// 設定不足でカタログから外れたプラグインの案内。無ければ空。
    pub(crate) fn catalog_notes(&self) -> &[String] {
        &self.catalog_notes
    }

    pub(crate) fn load_measurement(&self, patch: &str) -> Option<&PatchLoadMeasurement> {
        self.load_measurements.get(patch)
    }

    pub(crate) fn original(&self) -> Option<&str> {
        self.original.as_deref()
    }

    /// 直近に試聴した音色。取り消しで戻す必要があるかの判定に使う。
    pub(crate) fn previewed(&self) -> Option<&str> {
        self.previewed.as_deref()
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.filtered
            .get(self.cursor)
            .map(|index| self.all[*index].display())
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        match key.code {
            KeyCode::Esc => return PatchSelectAction::Cancel,
            KeyCode::Enter => {
                return match self.selected() {
                    Some(patch) => PatchSelectAction::Confirm(patch.to_string()),
                    None => PatchSelectAction::Cancel,
                };
            }
            _ => {}
        }
        if is_add_preset_key(key) {
            return self.add_query_as_preset();
        }
        if is_random_jump_key(key) {
            return self.random_jump();
        }
        match key.code {
            KeyCode::Left => {
                self.focus = match self.focus {
                    PatchSelectFocus::Groups | PatchSelectFocus::Presets => {
                        PatchSelectFocus::Groups
                    }
                    PatchSelectFocus::Patches => PatchSelectFocus::Presets,
                };
                return PatchSelectAction::Continue;
            }
            KeyCode::Right => {
                self.focus = match self.focus {
                    PatchSelectFocus::Groups => PatchSelectFocus::Presets,
                    PatchSelectFocus::Presets | PatchSelectFocus::Patches => {
                        PatchSelectFocus::Patches
                    }
                };
                return PatchSelectAction::Continue;
            }
            KeyCode::Up => return self.move_focused_cursor(-1),
            KeyCode::Down => return self.move_focused_cursor(1),
            KeyCode::PageUp => return self.move_focused_page(-1),
            KeyCode::PageDown => return self.move_focused_page(1),
            _ => {}
        }
        // 上記以外は絞り込み欄へ渡す。各空白区切り term を正規表現として扱う。
        if !text_input::apply_key_event_to_textarea(&mut self.query, key) {
            return PatchSelectAction::Continue;
        }
        self.refilter()
    }

    fn move_focused_cursor(&mut self, delta: isize) -> PatchSelectAction {
        match self.focus {
            PatchSelectFocus::Groups => self.move_group_cursor(delta),
            PatchSelectFocus::Presets => self.move_preset_cursor(delta),
            PatchSelectFocus::Patches => self.move_patch_cursor(delta),
        }
    }

    fn move_focused_page(&mut self, direction: isize) -> PatchSelectAction {
        self.move_focused_cursor(direction * PAGE_STEP)
    }

    fn move_group_cursor(&mut self, delta: isize) -> PatchSelectAction {
        let last = FilterGroup::ALL.len() - 1;
        let next = self.group_cursor.saturating_add_signed(delta).min(last);
        if next == self.group_cursor {
            return PatchSelectAction::Continue;
        }
        self.group_cursor = next;
        self.preset_cursor = 0;
        self.refilter()
    }

    fn move_preset_cursor(&mut self, delta: isize) -> PatchSelectAction {
        let last = self.presets().len().saturating_sub(1);
        let next = self.preset_cursor.saturating_add_signed(delta).min(last);
        if next == self.preset_cursor {
            return PatchSelectAction::Continue;
        }
        self.preset_cursor = next;
        self.refilter()
    }

    fn move_patch_cursor(&mut self, delta: isize) -> PatchSelectAction {
        if self.filtered.is_empty() {
            return PatchSelectAction::Continue;
        }
        let last = self.filtered.len() - 1;
        let next = self.cursor.saturating_add_signed(delta).min(last);
        if next == self.cursor {
            return PatchSelectAction::Continue;
        }
        self.cursor = next;
        self.preview_selected()
    }

    fn random_jump(&mut self) -> PatchSelectAction {
        self.focus = PatchSelectFocus::Patches;
        if self.filtered.len() < 2 {
            return PatchSelectAction::Continue;
        }
        let candidate = cmrt_tui_core::random::random_index(self.filtered.len() - 1)
            .expect("two or more filtered patches have another index");
        self.cursor = if candidate >= self.cursor {
            candidate + 1
        } else {
            candidate
        };
        self.preview_selected()
    }

    fn add_query_as_preset(&mut self) -> PatchSelectAction {
        let value = text_input::textarea_value(&self.query);
        let value = value.trim();
        let destination = self.selected_filter_group().user_destination();
        if value.is_empty()
            || !is_valid_condition(value)
            || patterns_for_role(destination, &self.user_presets)
                .iter()
                .any(|pattern| pattern == value)
        {
            return PatchSelectAction::Continue;
        }
        let previous_len = self.user_presets.len();
        self.user_presets
            .push((destination.key().to_string(), value.to_string()));
        self.user_presets = normalize_user_presets(std::mem::take(&mut self.user_presets));
        if self.user_presets.len() == previous_len {
            return PatchSelectAction::Continue;
        }
        self.role_index = build_role_index(&self.all, &self.user_presets);
        self.prepared_presets =
            PreparedPresets::build(&self.all, &self.user_presets, &self.role_index)
                .expect("validated preset regular expressions must compile");
        let preview = match self.refilter() {
            PatchSelectAction::Preview(patch) => Some(patch),
            _ => None,
        };
        PatchSelectAction::SaveUserPresets {
            presets: self.user_presets.clone(),
            preview,
        }
    }

    fn selected_filter_group(&self) -> FilterGroup {
        self.presets()[self.preset_cursor].group
    }

    fn refilter(&mut self) -> PatchSelectAction {
        self.update_filter();
        self.preview_selected()
    }

    fn update_filter(&mut self) {
        let selected = self.selected().map(str::to_string);
        let query = text_input::textarea_value(&self.query);
        let candidates = &self.presets()[self.preset_cursor].matches;
        let result = if query.trim().is_empty() {
            Ok(Arc::clone(candidates))
        } else {
            filter_candidates(&self.all, candidates, &query).map(Arc::from)
        };
        match result {
            Ok(filtered) => {
                self.filtered = filtered;
                self.filter_error = None;
            }
            Err(error) => {
                self.filtered = Arc::default();
                self.filter_error = Some(error);
            }
        }
        // 絞り込んだ結果に元の選択が残っていれば、そこへ留まる。
        self.cursor = selected
            .and_then(|selected| {
                self.filtered
                    .iter()
                    .position(|index| self.all[*index].display() == selected)
            })
            .unwrap_or(0);
    }

    fn preview_selected(&mut self) -> PatchSelectAction {
        let Some(patch) = self.selected() else {
            return PatchSelectAction::Continue;
        };
        if self.previewed.as_deref() == Some(patch) {
            return PatchSelectAction::Continue;
        }
        let patch = patch.to_string();
        self.previewed = Some(patch.clone());
        PatchSelectAction::Preview(patch)
    }
}

fn build_role_index(
    all: &[PatchCatalogEntry],
    user_presets: &[(String, String)],
) -> PatchRoleIndex {
    PatchRoleIndex::build(
        all.iter().map(|patch| PatchRoleInput {
            display: patch.display(),
            normalized_display: patch.normalized_display(),
            selector_category: patch.selector_category(),
        }),
        user_presets,
    )
}

fn is_add_preset_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('a')
}

fn is_random_jump_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('r')
}

/// このキーは音色選択を開く。
pub fn is_patch_select_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('t')
}

#[cfg(test)]
mod tests;
