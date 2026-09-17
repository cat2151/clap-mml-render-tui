use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
};

use super::PatchSelector;

/// Regex 欄の高さ（枠 2 行 + 入力 1 行）。
const QUERY_HEIGHT: u16 = 3;
/// 音色 pane の内側で、`Category | Patch | Load` の header が使う行数。
const PATCH_HEADER_HEIGHT: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatchSelectorLayout {
    pub(crate) popup: Rect,
    pub(crate) query: Option<Rect>,
    pub(crate) role_pane: Rect,
    pub(crate) preset_pane: Rect,
    pub(crate) patch_pane: Rect,
    pub(crate) role_list: Rect,
    pub(crate) preset_list: Rect,
    /// 音色 pane の内側の先頭行。`Category | Patch | Load` の header。
    pub(super) patch_header: Rect,
    /// 音色 pane の内側のうち、header を除いて音色の行が並ぶ矩形。
    pub(crate) patch_rows: Rect,
    pub(crate) hint: Rect,
}

impl PatchSelectorLayout {
    pub(crate) fn new(area: Rect, query_visible: bool) -> Self {
        let popup = cmrt_tui_core::ui::centered_rect(88, 76, area);
        let inner = Block::default().borders(Borders::ALL).inner(popup);
        let constraints = if query_visible {
            vec![
                Constraint::Length(QUERY_HEIGHT),
                Constraint::Min(1),
                Constraint::Length(1),
            ]
        } else {
            vec![Constraint::Min(1), Constraint::Length(1)]
        };
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);
        let (query, panes_index, hint_index) = if query_visible {
            (Some(vertical[0]), 1, 2)
        } else {
            (None, 0, 1)
        };
        let middle = vertical[panes_index];
        let role_width = (middle.width / 5).clamp(12, 22);
        let preset_width = (middle.width / 4).clamp(14, 30);
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(role_width),
                Constraint::Length(preset_width),
                Constraint::Min(12),
            ])
            .split(middle);
        let role_pane = panes[0];
        let preset_pane = panes[1];
        let patch_pane = panes[2];
        let patch_inner = Block::default().borders(Borders::ALL).inner(patch_pane);
        let patch_header = Rect {
            height: patch_inner.height.min(PATCH_HEADER_HEIGHT),
            ..patch_inner
        };
        let patch_rows = Rect {
            y: patch_inner.y.saturating_add(PATCH_HEADER_HEIGHT),
            height: patch_inner.height.saturating_sub(PATCH_HEADER_HEIGHT),
            ..patch_inner
        };
        Self {
            popup,
            query,
            role_pane,
            preset_pane,
            patch_pane,
            role_list: Block::default().borders(Borders::ALL).inner(role_pane),
            preset_list: Block::default().borders(Borders::ALL).inner(preset_pane),
            patch_header,
            patch_rows,
            hint: vertical[hint_index],
        }
    }

    pub(super) fn role_at(&self, selector: &PatchSelector, column: u16, row: u16) -> Option<usize> {
        item_at(
            self.role_list,
            selector.role_range(self).start,
            selector.roles().len(),
            column,
            row,
        )
    }

    pub(super) fn preset_at(
        &self,
        selector: &PatchSelector,
        column: u16,
        row: u16,
    ) -> Option<usize> {
        item_at(
            self.preset_list,
            selector.preset_range(self).start,
            selector.presets().len(),
            column,
            row,
        )
    }

    pub(super) fn patch_at(
        &self,
        selector: &PatchSelector,
        column: u16,
        row: u16,
    ) -> Option<usize> {
        item_at(
            self.patch_rows,
            selector.patch_range(self).start,
            selector.filtered_len(),
            column,
            row,
        )
    }
}

fn item_at(area: Rect, offset: usize, total: usize, column: u16, row: u16) -> Option<usize> {
    if !contains(area, column, row) {
        return None;
    }
    let index = offset + usize::from(row - area.y);
    (index < total).then_some(index)
}

pub(super) fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}
