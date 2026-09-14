//! MML オーバーレイから開く音色選択。
//!
//! 左から「Grid Sequencer 上の大分類」「正規表現プリセット」「音色」の 3 pane。
//! 手入力した正規表現とプリセットを AND で組み合わせる。選択そのものはここに閉じ、
//! 音を鳴らす/JSON へ永続化する処理は [`crate::state`] と host app に任せる。

mod filter;
mod keys;
mod navigation;
mod prepared;
mod presets;

use std::{cell::Cell, collections::BTreeMap, sync::Arc};

use cmrt_patches::{PatchRole, PatchRoleIndex};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::TextArea;

use cmrt_tui_core::{patch_load::PatchLoadMeasurement, text_input};

use crate::line_play::is_replay_key;
use crate::PatchCatalogEntry;

use filter::{filter_candidates, is_valid_condition};
use keys::{is_add_preset_key, is_filter_edit_trigger, is_random_jump_key};
pub(crate) use navigation::PatchSelectFocus;
use prepared::{build_role_index, PreparedPresets};
use presets::{normalize_user_presets, patterns_for_role, FilterGroup, FilterPreset};

pub use keys::is_patch_select_trigger;

/// 音色選択が呼び出し側へ求める処理。
pub(crate) enum PatchSelectAction {
    /// 表示が変わっただけ。
    Continue,
    /// この音色を試聴する。
    Preview(String),
    /// この音色へ差し替えて、入力欄の現在行をまるごと演奏する。
    ///
    /// 音色一覧の行は MML を持たない（[`PatchCatalogEntry`] は音色名・カテゴリ・
    /// load 時間だけ）ので、何を鳴らすかは呼び出し側が入力欄から決める。ここは
    /// 「どの音色で」だけを言う。
    PlayLine(String),
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
    /// 表示中かつ、編集中なら未確定の値も入る。
    query: TextArea<'a>,
    /// `Enter` で最後に確定した絞り込み。編集中の `Esc` はここへ戻す。
    committed_query: String,
    filter_editing: bool,
    filter_error: Option<String>,
    user_presets: Vec<(String, String)>,
    role_index: PatchRoleIndex,
    prepared_presets: PreparedPresets,
    group_cursor: usize,
    preset_cursor: usize,
    focus: PatchSelectFocus,
    /// 各 pane の描画時 scroll。viewport の高さは描画時にしか分からないため、
    /// UI が interior mutability で更新する。
    scroll_offsets: [Cell<usize>; 3],
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
        initial_role: Option<PatchRole>,
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
        let group_cursor = initial_role
            .and_then(|role| {
                FilterGroup::ALL
                    .iter()
                    .position(|group| group.role() == Some(role))
            })
            .unwrap_or(0);
        let filtered = Arc::clone(&prepared_presets.for_role(group_cursor)[0].matches);
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
            committed_query: String::new(),
            filter_editing: false,
            filter_error: None,
            user_presets,
            role_index,
            prepared_presets,
            group_cursor,
            preset_cursor: 0,
            focus: PatchSelectFocus::Patches,
            scroll_offsets: [Cell::new(0), Cell::new(0), Cell::new(0)],
            original: current.map(str::to_string),
            previewed: current.map(str::to_string),
            catalog_notes,
            load_measurements,
        })
    }

    pub(crate) fn query_textarea(&self) -> &TextArea<'a> {
        &self.query
    }

    pub(crate) fn filter_editing(&self) -> bool {
        self.filter_editing
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

    pub(crate) fn scroll_offset(&self, pane: PatchSelectFocus) -> usize {
        self.scroll_offsets[pane.index()].get()
    }

    pub(crate) fn set_scroll_offset(&self, pane: PatchSelectFocus, offset: usize) {
        self.scroll_offsets[pane.index()].set(offset);
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
        if self.filter_editing {
            return self.handle_filter_key(key);
        }
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
        if is_filter_edit_trigger(key) {
            self.filter_editing = true;
            self.query = text_input::new_single_line_textarea(&self.committed_query);
            return PatchSelectAction::Continue;
        }
        if is_replay_key(key) {
            return self.play_selected_line();
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
        PatchSelectAction::Continue
    }

    /// 絞り込み編集中は selector のキーを動かさず、入力欄だけを操作する。
    /// 候補は入力中にも更新するが、`Esc` なら編集開始時の確定値へ戻す。
    fn handle_filter_key(&mut self, key: KeyEvent) -> PatchSelectAction {
        match key.code {
            KeyCode::Esc => {
                self.filter_editing = false;
                self.query = text_input::new_single_line_textarea(&self.committed_query);
                return self.refilter();
            }
            KeyCode::Enter => {
                self.filter_editing = false;
                self.committed_query = text_input::textarea_value(&self.query);
                return PatchSelectAction::Continue;
            }
            _ => {}
        }
        if !text_input::apply_key_event_to_textarea(&mut self.query, key) {
            return PatchSelectAction::Continue;
        }
        self.refilter()
    }

    /// 選択中の音色で現在行を鳴らす。
    ///
    /// 試聴と同じく音源へこの音色を読み込ませるので、[`Self::previewed`] も進める。
    /// ここを飛ばすと、取り消し時に「読み込ませたのに元へ戻さない」が起きる。
    fn play_selected_line(&mut self) -> PatchSelectAction {
        let Some(patch) = self.selected() else {
            return PatchSelectAction::Continue;
        };
        let patch = patch.to_string();
        self.previewed = Some(patch.clone());
        PatchSelectAction::PlayLine(patch)
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

#[cfg(test)]
mod tests;
