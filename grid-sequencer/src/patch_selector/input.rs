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
        let layout = PatchSelectorLayout::new(terminal_area);
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
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.cancel_patch_selector();
                return;
            }
            KeyCode::Enter => {
                self.apply_patch_selection(ctx);
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
                selector.patch_cursor = selector.selected_category().patches.len() - 1;
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
