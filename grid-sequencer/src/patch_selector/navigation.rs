//! 3 pane の focus と、focus 中の pane のカーソル移動・random 抽選。

use cmrt_tui_core::random::random_index;

use super::PatchSelector;

const PAGE_STEP: isize = 10;

/// 上下キーがどの pane のカーソルを動かすか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatchPaneFocus {
    Role,
    Preset,
    Patches,
}

impl PatchPaneFocus {
    const ORDER: [Self; 3] = [Self::Role, Self::Preset, Self::Patches];

    pub(super) fn index(self) -> usize {
        Self::ORDER
            .iter()
            .position(|focus| *focus == self)
            .expect("PatchPaneFocus::ORDER contains every pane")
    }
}

impl PatchSelector {
    /// focus を左右へ動かす。端では止まる。音色は変えない。
    pub(super) fn move_focus(&mut self, delta: isize) {
        let last = PatchPaneFocus::ORDER.len() - 1;
        let next = self.focus.index().saturating_add_signed(delta).min(last);
        self.focus = PatchPaneFocus::ORDER[next];
    }

    /// focus 中の pane のカーソルを動かす。
    pub(super) fn move_focused_cursor(&mut self, delta: isize) {
        match self.focus {
            PatchPaneFocus::Role => self.move_role_cursor(delta),
            PatchPaneFocus::Preset => self.move_preset_cursor(delta),
            PatchPaneFocus::Patches => self.move_patch_cursor(delta),
        }
    }

    pub(super) fn move_focused_page(&mut self, direction: isize) {
        self.move_focused_cursor(direction * PAGE_STEP);
    }

    pub(super) fn move_focused_to_start(&mut self) {
        self.move_focused_cursor(isize::MIN);
    }

    pub(super) fn move_focused_to_end(&mut self) {
        self.move_focused_cursor(isize::MAX);
    }

    pub(super) fn move_role_cursor(&mut self, delta: isize) {
        let next = move_cursor(self.role_cursor, delta, self.roles().len());
        self.select_role(next);
    }

    pub(super) fn move_preset_cursor(&mut self, delta: isize) {
        let next = move_cursor(self.preset_cursor, delta, self.presets().len());
        self.select_preset(next);
    }

    pub(super) fn move_patch_cursor(&mut self, delta: isize) {
        let next = move_cursor(self.patch_cursor, delta, self.filtered.len());
        self.select_patch(next);
    }

    /// Role を選ぶ。Preset は `ALL` に戻して一覧を引き直す。
    pub(super) fn select_role(&mut self, index: usize) {
        if index >= self.roles().len() || index == self.role_cursor {
            return;
        }
        self.role_cursor = index;
        self.preset_cursor = 0;
        self.refilter();
    }

    pub(super) fn select_preset(&mut self, index: usize) {
        if index >= self.presets().len() || index == self.preset_cursor {
            return;
        }
        self.preset_cursor = index;
        self.refilter();
    }

    pub(super) fn select_patch(&mut self, index: usize) {
        if index < self.filtered.len() {
            self.patch_cursor = index;
        }
    }

    /// 今の一覧から現在行以外を 1 つ引く。focus は音色 pane へ移る。2 件未満なら何もしない。
    pub(super) fn select_random_patch(&mut self) {
        self.focus = PatchPaneFocus::Patches;
        if self.filtered.len() < 2 {
            return;
        }
        let Some(candidate) = random_index(self.filtered.len() - 1) else {
            return;
        };
        self.patch_cursor = if candidate >= self.patch_cursor {
            candidate + 1
        } else {
            candidate
        };
    }
}

fn move_cursor(current: usize, delta: isize, len: usize) -> usize {
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}
