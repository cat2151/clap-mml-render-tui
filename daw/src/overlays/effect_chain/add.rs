//! EFFECT CHAIN 追加 overlay（`x` → `a`）の category/kind/list 3 pane 状態。

use std::cell::Cell;

use cmrt_core::AudioEffectCatalog;
use cmrt_tui_core::text_filter;
use ratatui_textarea::TextArea;

/// 追加 overlay の selector から外す preset の値の前方一致。既存 chain の段はそのまま扱う
/// （selector から外すだけで catalog そのものからは外さない）。
const EXCLUDED_VALUE_PREFIXES: [&str; 1] = ["Reverb 1/"];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub(crate) enum EffectAddPane {
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
    pub(crate) fn prev(self) -> Self {
        match self {
            Self::Categories => Self::Categories,
            Self::Kinds => Self::Categories,
            Self::List => Self::Kinds,
        }
    }

    /// 1 つ右の pane（端では止まる）。
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Categories => Self::Kinds,
            Self::Kinds => Self::List,
            Self::List => Self::List,
        }
    }
}

/// 追加 overlay（`EffectChainAdd`）の状態。開くたびに catalog から組み直す。
pub(crate) struct DawEffectAddState {
    /// catalog の preset index。`EXCLUDED_VALUE_PREFIXES` を除いたもの。
    candidates: Vec<usize>,
    /// category pane の一覧（先頭は `all`）。
    pub(crate) categories: Vec<String>,
    pub(crate) category_cursor: usize,
    /// kind pane の一覧（先頭は `all`）。category pane の選択に合わせて組み直す。
    pub(crate) kinds: Vec<String>,
    pub(crate) kind_cursor: usize,
    /// list pane に出す catalog の preset index（`candidates` を category / kind / `query` で絞ったもの）。
    pub(crate) list: Vec<usize>,
    pub(crate) list_cursor: usize,
    pub(crate) focus: EffectAddPane,
    /// 各 pane の表示先頭。描画側が上下 30% の余白の規則で更新する。
    scroll_offsets: [Cell<usize>; 3],
    /// list の絞り込み条件（空なら絞り込みなし）。
    pub(crate) query: String,
    pub(crate) query_textarea: TextArea<'static>,
    pub(crate) query_before_input: String,
    pub(crate) filter_active: bool,
}

impl Default for DawEffectAddState {
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
        }
    }
}

impl DawEffectAddState {
    pub(crate) fn scroll_offset(&self, pane: EffectAddPane) -> &Cell<usize> {
        &self.scroll_offsets[pane.index()]
    }

    pub(crate) fn open(catalog: &AudioEffectCatalog) -> Self {
        let candidates: Vec<usize> = catalog
            .presets()
            .iter()
            .enumerate()
            .filter(|(_, preset)| {
                !EXCLUDED_VALUE_PREFIXES
                    .iter()
                    .any(|prefix| preset.value.starts_with(prefix))
            })
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
    pub(crate) fn rebuild_kinds(&mut self, catalog: &AudioEffectCatalog) {
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
    pub(crate) fn rebuild_list(&mut self, catalog: &AudioEffectCatalog) {
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

    /// `/`: 絞り込み編集を開始する。編集開始前の `query` を覚えておく（`Esc` で戻す用）。
    pub(crate) fn begin_filter(&mut self) {
        self.query_before_input = self.query.clone();
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.query);
        self.filter_active = true;
    }

    /// `Esc`: 編集開始前の `query` へ戻して編集を終える。
    pub(crate) fn cancel_filter(&mut self, catalog: &AudioEffectCatalog) {
        self.filter_active = false;
        if self.query == self.query_before_input {
            return;
        }
        self.query = self.query_before_input.clone();
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.query);
        self.rebuild_list(catalog);
    }
}
