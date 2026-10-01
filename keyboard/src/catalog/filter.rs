//! Patches pane の絞り込み条件。
//!
//! 規則は patch selector と同じ `cmrt_patch_select::filter_candidates` が単一ソース。
//! Role / Preset で絞った一覧の上にさらに掛ける。

use cmrt_patch_select::{filter_candidates, PatchCatalogEntry};

use super::{KeyboardPatchCatalog, PatchPaneFocus};

impl KeyboardPatchCatalog {
    /// 今掛かっている絞り込み条件。空なら絞り込みなし。
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// 条件を差し替えて一覧を絞り直す。
    ///
    /// 条件がコンパイルできなければ何も変えずに `Err`。音色が変わったときだけ `Ok(Some)`
    /// （今の音色が一覧に残ればそのまま、消えれば先頭）。
    pub(crate) fn set_filter(&mut self, condition: &str) -> Result<Option<String>, String> {
        let list = filter_candidates(&self.entries, self.preset_matches(), condition)?;
        let previous = self.selected_patch().map(str::to_string);
        self.filter = condition.to_string();
        self.list = list;
        Ok(self.refilter(previous))
    }

    /// 一覧がまだ読めていない catalog へ条件だけ引き継ぐ。次の `load` で掛かる。
    pub(crate) fn carry_filter_from(&mut self, other: &KeyboardPatchCatalog) {
        self.filter = other.filter.clone();
    }

    /// Role / Preset / 条件で絞る前の全音色。
    pub(crate) fn entries(&self) -> &[PatchCatalogEntry] {
        &self.entries
    }

    pub(crate) fn focus_patches(&mut self) {
        self.focus = PatchPaneFocus::Patches;
    }

    /// Role / Preset / 条件のどれかが変わった後に、Patches pane の一覧を作り直す。
    pub(super) fn rebuild_list(&mut self) {
        let matches = self.preset_matches();
        self.list = filter_candidates(&self.entries, matches, &self.filter)
            .unwrap_or_else(|_| matches.to_vec());
    }

    fn preset_matches(&self) -> &[usize] {
        self.presets()
            .get(self.preset_cursor)
            .map(|preset| &*preset.matches)
            .unwrap_or(&[])
    }
}
