//! 音色選択の一覧。
//!
//! 左から「Grid Sequencer 上の大分類」「正規表現プリセット」「音色」の 3 pane。
//! 手入力した正規表現とプリセットを AND で組み合わせる。選択そのものはここに閉じ、
//! 音を鳴らす処理は [`crate::PatchAuditionSelect`]、JSON へ永続化する処理は host app に任せる。

mod auto_reverb;
mod favorites;
mod filter;
mod input;
mod keys;
mod navigation;
mod open;
mod plugin_menu;
mod prepared;
mod presets;

use std::{cell::Cell, collections::BTreeMap, sync::Arc};

use cmrt_patches::PatchRoleIndex;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use cmrt_tui_core::{patch_load::PatchLoadMeasurement, text_input};

use crate::{auto_reverb::AutoReverbRules, PatchCatalogEntry};

pub(crate) use crate::plugin_menu::PluginMenu;
#[cfg(test)]
pub(crate) use crate::plugin_menu::PluginMode;
pub(crate) use auto_reverb::EffectList;
pub use auto_reverb::{AutoReverbHost, AutoReverbKey, AutoReverbPanel, AutoReverbStatus};
pub use filter::filter_candidates;
use filter::is_valid_condition;
use keys::{
    is_add_preset_key, is_filter_edit_trigger, is_plugin_menu_key, is_preview_key,
    is_random_jump_key,
};
pub(crate) use navigation::PatchSelectFocus;
pub use navigation::PAGE_STEP;
use open::prepare_presets;
pub use open::PatchSelectRequest;
use prepared::build_role_index;
pub use prepared::PreparedPresets;
use presets::{normalize_user_presets, patterns_for_role};
pub use presets::{prepare_user_presets, FilterGroup, FilterPreset};

pub(crate) use keys::is_patch_select_play_settings_trigger;

/// 音色選択が呼び出し側へ求める処理。
#[derive(Debug, PartialEq, Eq)]
pub enum PatchSelectAction {
    /// 表示が変わっただけ。
    Continue,
    /// この音色を試聴する。
    Preview(String),
    /// この音色へ差し替えて、行をまるごと演奏する。
    ///
    /// 音色一覧の行は MML を持たない（[`PatchCatalogEntry`] は音色名・カテゴリ・
    /// load 時間だけ）ので、何を鳴らすかは selector を持つ側が決める。ここは
    /// 「どの音色で」だけを言う。
    PlayLine(String),
    /// この音色で確定して閉じる。
    Confirm(String),
    /// ユーザー追加プリセットを JSON へ保存する。
    SaveUserPresets {
        presets: Vec<(String, String)>,
        preview: Option<String>,
    },
    /// auto reverb のルール（on/off を含む）を保存し、`preview` の音色を今の設定で鳴らし直す。
    ///
    /// 同じ音色でも鳴らし直す（掛ける reverb が変わったため）。
    SaveAutoReverb {
        rules: AutoReverbRules,
        preview: Option<String>,
    },
    /// 取り消して閉じる。開いたときの音色へ戻す。
    Cancel,
}

pub struct PatchSelect<'a> {
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
    /// Role / Preset を固定し、設定を返さない選択専用モード。
    /// 返す試聴は Space の [`PatchSelectAction::PlayLine`] だけで、候補移動では返さない。
    drum_kit_only: bool,
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
    /// 音色 favorite。登録が新しい順。
    favorites: Vec<String>,
    /// host が auto reverb を扱うときだけ `Some`。
    auto_reverb: Option<AutoReverbPanel>,
    /// plugin の solo / mute menu。開いている間だけ `Some`。
    plugin_menu: Option<PluginMenu>,
}

impl<'a> PatchSelect<'a> {
    pub fn drum_kit_only(&self) -> bool {
        self.drum_kit_only
    }

    pub fn query_textarea(&self) -> &TextArea<'a> {
        &self.query
    }

    /// Regex 欄を編集中か。編集中は全キーを [`Self::handle_key`] へ渡すこと。
    pub fn filter_editing(&self) -> bool {
        self.filter_editing
    }

    pub(crate) fn filter_error(&self) -> Option<&str> {
        self.filter_error.as_deref()
    }

    pub(crate) fn groups(&self) -> &[FilterGroup] {
        &FilterGroup::ALL
    }

    pub fn group_cursor(&self) -> usize {
        self.group_cursor
    }

    pub fn presets(&self) -> &[FilterPreset] {
        self.prepared_presets.for_role(self.group_cursor)
    }

    pub fn preset_cursor(&self) -> usize {
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

    pub fn filtered_len(&self) -> usize {
        self.filtered.len()
    }

    /// 音色 pane のカーソル（絞り込み後の一覧での位置）。
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 絞り込み後の一覧で `index` 番目の音色名。
    pub fn filtered_display(&self, index: usize) -> Option<&str> {
        self.filtered
            .get(index)
            .map(|index| self.all[*index].display())
    }

    pub(crate) fn total(&self) -> usize {
        self.all.len()
    }

    /// 設定不足でカタログから外れたプラグインの案内。無ければ空。
    pub(crate) fn catalog_notes(&self) -> &[String] {
        &self.catalog_notes
    }

    pub fn load_measurement(&self, patch: &str) -> Option<&PatchLoadMeasurement> {
        self.load_measurements.get(patch)
    }

    pub(crate) fn original(&self) -> Option<&str> {
        self.original.as_deref()
    }

    /// 直近に試聴した音色。取り消しで戻す必要があるかの判定に使う。
    pub(crate) fn previewed(&self) -> Option<&str> {
        self.previewed.as_deref()
    }

    pub fn selected(&self) -> Option<&str> {
        self.filtered
            .get(self.cursor)
            .map(|index| self.all[*index].display())
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
        if self.drum_kit_only {
            return PatchSelectAction::Continue;
        }
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
