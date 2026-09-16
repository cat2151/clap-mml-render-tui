//! 3 pane の focus と、focus 中の pane のカーソル移動・random 抽選。

use rand::seq::SliceRandom;

use cmrt_mml_overlay::FilterGroup;

use super::KeyboardPatchCatalog;

/// 上下キーがどの pane のカーソルを動かすか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchPaneFocus {
    Role,
    Preset,
    Patches,
}

impl PatchPaneFocus {
    const ORDER: [Self; 3] = [Self::Role, Self::Preset, Self::Patches];

    fn index(self) -> usize {
        Self::ORDER
            .iter()
            .position(|focus| *focus == self)
            .expect("PatchPaneFocus::ORDER contains every pane")
    }
}

impl KeyboardPatchCatalog {
    /// focus を左右へ動かす。端では止まる。音色は変えない。
    pub(crate) fn move_focus(&mut self, delta: isize) {
        let last = PatchPaneFocus::ORDER.len() - 1;
        let next = self.focus.index().saturating_add_signed(delta).min(last);
        self.focus = PatchPaneFocus::ORDER[next];
    }

    /// focus 中の pane のカーソルを動かす。音色が変わったときだけ `Some`。
    pub(crate) fn move_focused_cursor(&mut self, delta: isize) -> Option<String> {
        match self.focus {
            PatchPaneFocus::Role => self.move_role_cursor(delta),
            PatchPaneFocus::Preset => self.move_preset_cursor(delta),
            PatchPaneFocus::Patches => self.move_patch_cursor(delta),
        }
    }

    pub(crate) fn move_focused_to_start(&mut self) -> Option<String> {
        self.move_focused_cursor(isize::MIN)
    }

    pub(crate) fn move_focused_to_end(&mut self) -> Option<String> {
        self.move_focused_cursor(isize::MAX)
    }

    fn move_role_cursor(&mut self, delta: isize) -> Option<String> {
        let last = FilterGroup::ALL.len() - 1;
        let next = self.role_cursor.saturating_add_signed(delta).min(last);
        if next == self.role_cursor {
            return None;
        }
        let previous = self.selected_patch().map(str::to_string);
        self.role_cursor = next;
        self.preset_cursor = 0;
        self.refilter(previous)
    }

    fn move_preset_cursor(&mut self, delta: isize) -> Option<String> {
        let last = self.presets().len().saturating_sub(1);
        let next = self.preset_cursor.saturating_add_signed(delta).min(last);
        if next == self.preset_cursor {
            return None;
        }
        let previous = self.selected_patch().map(str::to_string);
        self.preset_cursor = next;
        self.refilter(previous)
    }

    fn move_patch_cursor(&mut self, delta: isize) -> Option<String> {
        let list_len = self.list().len();
        if list_len == 0 {
            return None;
        }
        let Some(cursor) = self.patch_cursor else {
            return self.select(0);
        };
        let next = cursor.saturating_add_signed(delta).min(list_len - 1);
        if next == cursor {
            return None;
        }
        self.select(next)
    }

    /// 一覧が変わった後の音色。元の音色が新しい一覧に残っていればそこに留まり、無ければ先頭。
    fn refilter(&mut self, previous: Option<String>) -> Option<String> {
        let position = self.position_of(previous.as_deref());
        if self.list().is_empty() {
            self.patch_cursor = None;
            return None;
        }
        let next = position.unwrap_or(0);
        let patch = self.select(next)?;
        (Some(patch.as_str()) != previous.as_deref()).then_some(patch)
    }

    /// 現在の一覧から、現在の音色以外を 1 つ引く。focus は音色 pane へ移る。
    ///
    /// 同じ音色を続けて引かないよう、一覧を 1 周するまで抽選済みを除く。
    pub(crate) fn select_random_patch(&mut self) -> Option<String> {
        self.focus = PatchPaneFocus::Patches;
        let deck_key = (self.role_cursor, self.preset_cursor);
        if self.random_deck_key != Some(deck_key) {
            self.clear_random_deck();
            self.random_deck_key = Some(deck_key);
        }
        let current = self.patch_cursor;
        self.random_remaining
            .retain(|index| Some(*index) != current);
        if self.random_remaining.is_empty() {
            self.random_remaining = (0..self.list().len())
                .filter(|index| Some(*index) != current)
                .collect();
            self.random_remaining.shuffle(&mut rand::rng());
        }
        let next = self.random_remaining.pop()?;
        self.select(next)
    }

    pub(super) fn select(&mut self, cursor: usize) -> Option<String> {
        let patch = self
            .list()
            .get(cursor)
            .map(|index| self.entries[*index].display().to_string())?;
        self.patch_cursor = Some(cursor);
        self.patch_table_state.select(Some(cursor));
        Some(patch)
    }
}
