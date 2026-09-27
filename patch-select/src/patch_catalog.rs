//! 音色 selector が表示・検索・整列に使う catalog。

mod from_patch_load;

pub use from_patch_load::{host_patch_catalog, HostPatchCatalog};

/// selector が受け取る、plugin 非依存の音色一覧スナップショット。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PatchCatalogSnapshot {
    /// バックグラウンド収集中。selector を開く要求は完了後の open 予約になる。
    #[default]
    Loading,
    /// 収集済みのselector行。空なら選べる音色がない。
    Ready(Vec<PatchCatalogEntry>),
    /// 収集に失敗した理由。selector を開こうとしたときに表示する。
    Error(String),
}

/// plugin固有情報をserver側で解釈済みにした、selector向けの中立な表現。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchCatalogEntry {
    display: String,
    normalized_display: String,
    plugin_sort_key: String,
    selector_category: Option<String>,
    normalized_selector_category: Option<String>,
    /// この行へまとめた同じ音の patch の件数（自身を含む）。まとめていなければ 1。
    merged_count: usize,
    /// まとめた patch の名前。別名でも検索できるよう、絞り込みの対象にする。
    merged_names: Vec<String>,
}

impl PatchCatalogEntry {
    pub fn from_display(display: String) -> Self {
        let normalized_display = display.to_lowercase();
        Self::new(display, normalized_display, String::new(), None)
    }

    pub fn new(
        display: String,
        normalized_display: String,
        plugin_sort_key: String,
        selector_category: Option<String>,
    ) -> Self {
        let selector_category = selector_category.filter(|category| !category.trim().is_empty());
        let normalized_selector_category = selector_category.as_deref().map(str::to_lowercase);
        Self {
            display,
            normalized_display,
            plugin_sort_key: plugin_sort_key.to_lowercase(),
            selector_category,
            normalized_selector_category,
            merged_count: 1,
            merged_names: Vec::new(),
        }
    }

    /// 同じ音の patch をこの行へまとめた件数と名前を付ける。
    pub fn with_merged(mut self, count: usize, names: Vec<String>) -> Self {
        self.merged_count = count;
        self.merged_names = names;
        self
    }

    pub fn merged_count(&self) -> usize {
        self.merged_count
    }

    pub(crate) fn merged_names(&self) -> &[String] {
        &self.merged_names
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub(crate) fn normalized_display(&self) -> &str {
        &self.normalized_display
    }

    pub fn plugin_sort_key(&self) -> &str {
        &self.plugin_sort_key
    }

    pub fn selector_category(&self) -> Option<&str> {
        self.selector_category.as_deref()
    }

    /// Categoryなしを末尾へ送り、残りをCategory / plugin / patch名の順で整列する。
    pub(crate) fn selector_sort_key(&self) -> (bool, &str, &str, &str) {
        (
            self.normalized_selector_category.is_none(),
            self.normalized_selector_category.as_deref().unwrap_or(""),
            &self.plugin_sort_key,
            &self.normalized_display,
        )
    }

    pub(crate) fn normalized_selector_category(&self) -> Option<&str> {
        self.normalized_selector_category.as_deref()
    }
}

/// selector が見せる順に整列する。Category 無しを末尾へ送り、残りを Category / plugin / patch 名の順にする。
pub fn sort_for_selector(entries: &mut [PatchCatalogEntry]) {
    entries.sort_by(|left, right| left.selector_sort_key().cmp(&right.selector_sort_key()));
}
