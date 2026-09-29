//! 追加・差し替え overlay の category/kind/list 3 pane 状態。

use std::cell::Cell;

use cmrt_core::AudioEffectCatalog;
use cmrt_patch_select::auto_reverb::is_selectable_effect_preset;
use cmrt_tui_core::text_filter;
use ratatui_textarea::TextArea;
use serde_json::Value;

mod input;

pub use input::AddKeyAction;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum EffectAddPane {
    Categories,
    Kinds,
    #[default]
    List,
}

impl EffectAddPane {
    fn index(self) -> usize {
        match self {
            Self::Categories => 0,
            Self::Kinds => 1,
            Self::List => 2,
        }
    }

    /// 1 つ左の pane（端では止まる）。
    pub fn prev(self) -> Self {
        match self {
            Self::Categories => Self::Categories,
            Self::Kinds => Self::Categories,
            Self::List => Self::Kinds,
        }
    }

    /// 1 つ右の pane（端では止まる）。
    pub fn next(self) -> Self {
        match self {
            Self::Categories => Self::Kinds,
            Self::Kinds => Self::List,
            Self::List => Self::List,
        }
    }
}

/// 追加・差し替え overlay の状態。開くたびに catalog から組み直す。
pub struct EffectAddState {
    /// catalog の preset index。`is_selectable_effect_preset` が外すものを除いたもの。
    candidates: Vec<usize>,
    /// category pane の一覧（先頭は `all`）。
    pub categories: Vec<String>,
    pub category_cursor: usize,
    /// kind pane の一覧（先頭は `all`）。category pane の選択に合わせて組み直す。
    pub kinds: Vec<String>,
    pub kind_cursor: usize,
    /// list pane に出す catalog の preset index（`candidates` を category / kind / `query` で絞ったもの）。
    pub list: Vec<usize>,
    pub list_cursor: usize,
    pub focus: EffectAddPane,
    /// 各 pane の表示先頭。描画側が上下 30% の余白の規則で更新する。
    scroll_offsets: [Cell<usize>; 3],
    /// list の絞り込み条件（空なら絞り込みなし）。
    pub query: String,
    pub query_textarea: TextArea<'static>,
    pub query_before_input: String,
    pub filter_active: bool,
    /// `Some(i)` なら chain の `i` 段目を差し替える（`r`）。`None` なら末尾へ追加する（`a`）。
    pub replace_target: Option<usize>,
}

impl Default for EffectAddState {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            categories: Vec::new(),
            category_cursor: 0,
            kinds: Vec::new(),
            kind_cursor: 0,
            list: Vec::new(),
            list_cursor: 0,
            focus: EffectAddPane::List,
            scroll_offsets: [Cell::new(0), Cell::new(0), Cell::new(0)],
            query: String::new(),
            query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            query_before_input: String::new(),
            filter_active: false,
            replace_target: None,
        }
    }
}

impl EffectAddState {
    pub fn scroll_offset(&self, pane: EffectAddPane) -> &Cell<usize> {
        &self.scroll_offsets[pane.index()]
    }

    pub fn open(catalog: &AudioEffectCatalog) -> Self {
        let candidates: Vec<usize> = catalog
            .presets()
            .iter()
            .enumerate()
            .filter(|(_, preset)| is_selectable_effect_preset(preset))
            .map(|(index, _)| index)
            .collect();
        let mut categories: Vec<String> = candidates
            .iter()
            .map(|&index| catalog.presets()[index].category.clone())
            .collect();
        categories.sort();
        categories.dedup();
        categories.insert(0, "all".to_string());

        let mut state = Self {
            candidates,
            categories,
            ..Self::default()
        };
        state.rebuild_kinds(catalog);
        state
    }

    /// `all` 以外を選んでいれば category 名を返す。
    fn selected_category(&self) -> Option<&str> {
        self.categories
            .get(self.category_cursor)
            .map(String::as_str)
            .filter(|category| *category != "all")
    }

    /// `all` 以外を選んでいれば kind 名を返す。
    fn selected_kind(&self) -> Option<&str> {
        self.kinds
            .get(self.kind_cursor)
            .map(String::as_str)
            .filter(|kind| *kind != "all")
    }

    /// category pane の選択に合わせて kind pane を組み直し、kind カーソルを 0（`all`）へ戻す。
    /// 続けて list も組み直す。
    pub fn rebuild_kinds(&mut self, catalog: &AudioEffectCatalog) {
        let category = self.selected_category();
        let mut kinds: Vec<String> = self
            .candidates
            .iter()
            .map(|&index| &catalog.presets()[index])
            .filter(|preset| category.is_none_or(|category| preset.category == category))
            .map(|preset| preset.kind.clone())
            .collect();
        kinds.sort();
        kinds.dedup();
        kinds.insert(0, "all".to_string());
        self.kinds = kinds;
        self.kind_cursor = 0;
        self.rebuild_list(catalog);
    }

    /// category / kind pane の選択と `query` で list を絞り直し、list カーソルを 0 へ戻す。
    ///
    /// `query` が正規表現としてコンパイルできない（打鍵途中の `(` など）場合は、
    /// list・list カーソルとも直前の状態のまま何もしない（全消えにしない）。
    pub fn rebuild_list(&mut self, catalog: &AudioEffectCatalog) {
        let condition = match text_filter::compile_condition(&self.query) {
            Ok(condition) => condition,
            Err(_) => return,
        };
        let category = self.selected_category();
        let kind = self.selected_kind();
        self.list = self
            .candidates
            .iter()
            .copied()
            .filter(|&index| {
                let preset = &catalog.presets()[index];
                if category.is_some_and(|category| preset.category != category) {
                    return false;
                }
                if kind.is_some_and(|kind| preset.kind != kind) {
                    return false;
                }
                let plugin_name = catalog
                    .plugin(&preset.plugin)
                    .map(|plugin| plugin.name.as_str())
                    .unwrap_or("");
                text_filter::matches_any_field(
                    &condition,
                    &[
                        preset.name.as_str(),
                        preset.category.as_str(),
                        preset.kind.as_str(),
                        plugin_name,
                    ],
                )
            })
            .collect();
        self.list_cursor = 0;
    }

    /// `r`: list pane へ focus し、list カーソルを今の位置以外のランダムな候補へ動かす。
    /// 動いたら `true`。list が 1 件以下なら動かさない。
    pub fn random_jump_list(&mut self) -> bool {
        self.focus = EffectAddPane::List;
        let Some(candidate) =
            cmrt_tui_core::random::random_index(self.list.len().saturating_sub(1))
        else {
            return false;
        };
        self.list_cursor = if candidate >= self.list_cursor {
            candidate + 1
        } else {
            candidate
        };
        true
    }

    /// `/`: 絞り込み編集を開始する。編集開始前の `query` を覚えておく（`Esc` で戻す用）。
    pub fn begin_filter(&mut self) {
        self.query_before_input = self.query.clone();
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.query);
        self.filter_active = true;
    }

    /// list `index` の preset を chain の 1 段にした値。
    pub fn preset_stage(
        &self,
        catalog: Option<&AudioEffectCatalog>,
        index: usize,
    ) -> Option<Value> {
        let preset_index = *self.list.get(index)?;
        catalog?
            .presets()
            .get(preset_index)
            .map(cmrt_core::AudioEffectPreset::json_element)
    }

    /// list カーソルの preset（catalog の index）。list が空なら `None`。
    pub fn candidate(&self) -> Option<usize> {
        self.list.get(self.list_cursor).copied()
    }

    /// `Esc`: 編集開始前の `query` へ戻して編集を終える。
    pub fn cancel_filter(&mut self, catalog: &AudioEffectCatalog) {
        self.filter_active = false;
        if self.query == self.query_before_input {
            return;
        }
        self.query = self.query_before_input.clone();
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.query);
        self.rebuild_list(catalog);
    }
}

#[cfg(test)]
mod tests;
