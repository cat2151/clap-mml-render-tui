use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
};

use super::PatchSelector;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatchSelectorLayout {
    pub(crate) popup: Rect,
    pub(crate) category_pane: Rect,
    pub(crate) patch_pane: Rect,
    pub(crate) category_list: Rect,
    pub(crate) patch_list: Rect,
    pub(crate) hint: Rect,
}

impl PatchSelectorLayout {
    pub(crate) fn new(area: Rect) -> Self {
        let popup = cmrt_tui_core::ui::centered_rect(88, 76, area);
        let inner = Block::default().borders(Borders::ALL).inner(popup);
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
            .split(vertical[0]);
        let category_pane = panes[0];
        let patch_pane = panes[1];
        Self {
            popup,
            category_pane,
            patch_pane,
            category_list: Block::default().borders(Borders::ALL).inner(category_pane),
            patch_list: Block::default().borders(Borders::ALL).inner(patch_pane),
            hint: vertical[1],
        }
    }

    pub(super) fn category_at(
        &self,
        selector: &PatchSelector,
        column: u16,
        row: u16,
    ) -> Option<usize> {
        item_at(
            self.category_list,
            selector.category_range(self).start,
            selector.categories.len(),
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
            self.patch_list,
            selector.patch_range(self).start,
            selector.selected_category().patches.len(),
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
