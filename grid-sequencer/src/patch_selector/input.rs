//! patch selector 表示中の mouse / keyboard 入力さばき。

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::{contains, PatchSelectorLayout};
use crate::{GridSequencerContext, GridSequencerScreen};

impl GridSequencerScreen {
    pub(crate) fn handle_patch_selector_mouse(
        &mut self,
        event: MouseEvent,
        terminal_area: Rect,
        ctx: &GridSequencerContext<'_>,
    ) {
        let filter_visible = self
            .patch_selector
            .as_ref()
            .is_some_and(super::PatchSelector::filter_visible);
        let layout = PatchSelectorLayout::new(terminal_area, filter_visible);
        match event.kind {
            MouseEventKind::Down(MouseButton::Right | MouseButton::Middle) => {
                self.cancel_patch_selector();
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Some(selector) = self.patch_selector.as_mut() else {
                    return;
                };
                if let Some(index) = layout.category_at(selector, event.column, event.row) {
                    selector.select_category(index);
                    self.preview_patch_selection(ctx);
                } else if let Some(index) = layout.patch_at(selector, event.column, event.row) {
                    selector.select_patch(index);
                    self.apply_patch_selection(ctx);
                } else if layout
                    .filter
                    .is_some_and(|area| contains(area, event.column, event.row))
                {
                    // textarea が keyboard focus を持ったままなので、clickでは状態を変えない。
                } else {
                    self.cancel_patch_selector();
                }
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let delta = if matches!(event.kind, MouseEventKind::ScrollUp) {
                    -1
                } else {
                    1
                };
                let Some(selector) = self.patch_selector.as_mut() else {
                    return;
                };
                if contains(layout.category_pane, event.column, event.row) {
                    selector.move_category(delta);
                } else if contains(layout.patch_pane, event.column, event.row) {
                    selector.move_patch(delta);
                }
                self.preview_patch_selection(ctx);
            }
            MouseEventKind::Up(_)
            | MouseEventKind::Drag(_)
            | MouseEventKind::Moved
            | MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight => {}
        }
    }

    pub(crate) fn handle_patch_selector_key(
        &mut self,
        key: KeyEvent,
        ctx: &GridSequencerContext<'_>,
    ) {
        if self
            .patch_selector
            .as_ref()
            .is_some_and(|selector| selector.filter_active)
        {
            let preview = match key.code {
                KeyCode::Esc => {
                    self.patch_selector
                        .as_mut()
                        .expect("filter belongs to an open selector")
                        .cancel_filter_input();
                    false
                }
                KeyCode::Enter => {
                    self.patch_selector
                        .as_mut()
                        .expect("filter belongs to an open selector")
                        .confirm_filter_input();
                    true
                }
                _ => {
                    let selector = self
                        .patch_selector
                        .as_mut()
                        .expect("filter belongs to an open selector");
                    selector.sync_filter_textarea();
                    selector.apply_filter_key(key);
                    false
                }
            };
            if preview {
                self.preview_patch_selection(ctx);
            }
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.cancel_patch_selector();
                return;
            }
            KeyCode::Enter => {
                self.apply_patch_selection(ctx);
                return;
            }
            KeyCode::Char('/') => {
                if let Some(selector) = self.patch_selector.as_mut() {
                    selector.start_filter_input();
                }
                return;
            }
            _ => {}
        }
        let Some(selector) = self.patch_selector.as_mut() else {
            return;
        };
        let navigated = match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                selector.move_category(-1);
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                selector.move_category(1);
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selector.move_patch(-1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selector.move_patch(1);
                true
            }
            KeyCode::PageUp => {
                selector.move_patch(-10);
                true
            }
            KeyCode::PageDown => {
                selector.move_patch(10);
                true
            }
            KeyCode::Home => {
                selector.patch_cursor = 0;
                true
            }
            KeyCode::End => {
                selector.patch_cursor =
                    selector.selected_category().patches.len().saturating_sub(1);
                true
            }
            KeyCode::Char('r') => {
                selector.select_random_patch();
                true
            }
            _ => false,
        };
        if navigated {
            self.preview_patch_selection(ctx);
        }
    }
}
