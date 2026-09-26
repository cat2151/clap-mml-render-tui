//! 3 pane の focus と上下移動。

use super::*;

const PAGE_STEP: isize = 10;

/// 左右キーでどの pane のカーソルを上下移動するか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatchSelectFocus {
    Groups,
    Presets,
    Patches,
}

impl PatchSelectFocus {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Groups => 0,
            Self::Presets => 1,
            Self::Patches => 2,
        }
    }
}

impl PatchSelect<'_> {
    pub(super) fn move_focused_cursor(&mut self, delta: isize) -> PatchSelectAction {
        match self.focus {
            PatchSelectFocus::Groups => self.move_group_cursor(delta),
            PatchSelectFocus::Presets => self.move_preset_cursor(delta),
            PatchSelectFocus::Patches => self.move_patch_cursor(delta),
        }
    }

    pub(super) fn move_focused_page(&mut self, direction: isize) -> PatchSelectAction {
        self.move_focused_cursor(direction * PAGE_STEP)
    }

    pub(super) fn move_focused_to_start(&mut self) -> PatchSelectAction {
        self.move_focused_cursor(isize::MIN)
    }

    pub(super) fn move_focused_to_end(&mut self) -> PatchSelectAction {
        self.move_focused_cursor(isize::MAX)
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
}
