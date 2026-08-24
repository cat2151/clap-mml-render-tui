//! MML patch selector が表示・検索・整列に使うcatalogの1行。

/// plugin固有情報をserver側で解釈済みにした、selector向けの中立な表現。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchCatalogEntry {
    display: String,
    normalized_display: String,
    plugin_sort_key: String,
    selector_category: Option<String>,
    normalized_selector_category: Option<String>,
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
        }
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
