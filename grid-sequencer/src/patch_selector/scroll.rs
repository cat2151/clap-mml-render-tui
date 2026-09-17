//! 各 pane の表示範囲。カーソルを上下 30% の余白の内側に保ち、余白に入るまで scroll しない。

use std::ops::Range;

use cmrt_mml_overlay::ui::scroll_offset;

use super::{PatchPaneFocus, PatchSelector, PatchSelectorLayout};

impl PatchSelector {
    pub(crate) fn role_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        self.scrolled_range(
            PatchPaneFocus::Role,
            self.roles().len(),
            self.role_cursor,
            usize::from(layout.role_list.height),
        )
    }

    pub(crate) fn preset_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        self.scrolled_range(
            PatchPaneFocus::Preset,
            self.presets().len(),
            self.preset_cursor,
            usize::from(layout.preset_list.height),
        )
    }

    pub(crate) fn patch_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        self.scrolled_range(
            PatchPaneFocus::Patches,
            self.filtered.len(),
            self.patch_cursor,
            usize::from(layout.patch_rows.height),
        )
    }

    /// カーソルを上下 30% の余白の内側に保つ範囲。余白の規則は MML overlay と同じ。
    fn scrolled_range(
        &self,
        pane: PatchPaneFocus,
        total: usize,
        cursor: usize,
        height: usize,
    ) -> Range<usize> {
        let current = &self.scroll_offsets[pane.index()];
        let offset = scroll_offset(cursor, total, height, current.get());
        current.set(offset);
        offset..offset + height.min(total)
    }
}
