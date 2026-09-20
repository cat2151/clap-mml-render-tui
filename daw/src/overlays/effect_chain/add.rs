//! EFFECT CHAIN 追加 overlay（`x` → `a`）の role/list 2 pane 状態。

use cmrt_core::AudioEffectCatalog;
use cmrt_tui_core::text_filter;
use ratatui_textarea::TextArea;

/// 追加 overlay の selector から外す role。既存 chain の段はそのまま扱う
/// （selector から外すだけで catalog そのものからは外さない）。
const EXCLUDED_ROLES: [&str; 1] = ["Reverb 1"];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub(crate) enum EffectAddPane {
    Roles,
    #[default]
    List,
}

/// 追加 overlay（`EffectChainAdd`）の状態。開くたびに catalog から組み直す。
pub(crate) struct DawEffectAddState {
    /// catalog の preset index。`EXCLUDED_ROLES` を除いたもの。
    candidates: Vec<usize>,
    /// role pane の一覧（先頭は `all`）。
    pub(crate) roles: Vec<String>,
    pub(crate) role_cursor: usize,
    /// list pane に出す catalog の preset index（`candidates` を role と `query` で絞ったもの）。
    pub(crate) list: Vec<usize>,
    pub(crate) list_cursor: usize,
    pub(crate) focus: EffectAddPane,
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
            roles: Vec::new(),
            role_cursor: 0,
            list: Vec::new(),
            list_cursor: 0,
            focus: EffectAddPane::List,
            query: String::new(),
            query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            query_before_input: String::new(),
            filter_active: false,
        }
    }
}

impl DawEffectAddState {
    pub(crate) fn open(catalog: &AudioEffectCatalog) -> Self {
        let candidates: Vec<usize> = catalog
            .presets()
            .iter()
            .enumerate()
            .filter(|(_, preset)| !EXCLUDED_ROLES.contains(&preset.role.as_str()))
            .map(|(index, _)| index)
            .collect();
        let mut roles: Vec<String> = candidates
            .iter()
            .map(|&index| catalog.presets()[index].role.clone())
            .collect();
        roles.sort();
        roles.dedup();
        roles.insert(0, "all".to_string());

        let mut state = Self {
            candidates,
            roles,
            ..Self::default()
        };
        state.rebuild_list(catalog);
        state
    }

    /// `all` 以外を選んでいれば role 名を返す。
    fn selected_role(&self) -> Option<&str> {
        self.roles
            .get(self.role_cursor)
            .map(String::as_str)
            .filter(|role| *role != "all")
    }

    /// role pane の選択と `query` で list を絞り直し、list カーソルを 0 へ戻す。
    ///
    /// `query` が正規表現としてコンパイルできない（打鍵途中の `(` など）場合は、
    /// list・list カーソルとも直前の状態のまま何もしない（全消えにしない）。
    pub(crate) fn rebuild_list(&mut self, catalog: &AudioEffectCatalog) {
        let condition = match text_filter::compile_condition(&self.query) {
            Ok(condition) => condition,
            Err(_) => return,
        };
        let role = self.selected_role();
        self.list = self
            .candidates
            .iter()
            .copied()
            .filter(|&index| {
                let preset = &catalog.presets()[index];
                if role.is_some_and(|role| preset.role != role) {
                    return false;
                }
                let plugin_name = catalog
                    .plugin(&preset.plugin)
                    .map(|plugin| plugin.name.as_str())
                    .unwrap_or("");
                text_filter::matches_any_field(
                    &condition,
                    &[preset.name.as_str(), preset.role.as_str(), plugin_name],
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
