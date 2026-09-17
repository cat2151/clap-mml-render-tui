//! patch selector 表示中の mouse / keyboard 入力さばき。

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::{contains, PatchPaneFocus, PatchSelectorLayout};
use crate::{GridSequencerContext, GridSequencerScreen};

impl GridSequencerScreen {
    pub(crate) fn handle_patch_selector_mouse(
        &mut self,
        event: MouseEvent,
        terminal_area: Rect,
        ctx: &GridSequencerContext<'_>,
    ) {
        let query_visible = self
            .patch_selector
            .as_ref()
            .is_some_and(super::PatchSelector::filter_visible);
        let layout = PatchSelectorLayout::new(terminal_area, query_visible);
        match event.kind {
            MouseEventKind::Down(MouseButton::Right | MouseButton::Middle) => {
                self.cancel_patch_selector();
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Some(selector) = self.patch_selector.as_mut() else {
                    return;
                };
                if let Some(index) = layout.role_at(selector, event.column, event.row) {
                    selector.select_role(index);
                    selector.focus = PatchPaneFocus::Role;
                    self.preview_patch_selection(ctx);
                } else if let Some(index) = layout.preset_at(selector, event.column, event.row) {
                    selector.select_preset(index);
                    selector.focus = PatchPaneFocus::Preset;
                    self.preview_patch_selection(ctx);
                } else if let Some(index) = layout.patch_at(selector, event.column, event.row) {
                    selector.select_patch(index);
                    self.apply_patch_selection(ctx);
                } else if contains(layout.patch_header, event.column, event.row)
                    || layout
                        .query
                        .is_some_and(|area| contains(area, event.column, event.row))
                {
                    // header は選べる行ではない。Regex 欄は textarea が keyboard focus を
                    // 持ったままなので、click では状態を変えない。
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
                if contains(layout.role_pane, event.column, event.row) {
                    selector.move_role_cursor(delta);
                } else if contains(layout.preset_pane, event.column, event.row) {
                    selector.move_preset_cursor(delta);
                } else if contains(layout.patch_pane, event.column, event.row) {
                    selector.move_patch_cursor(delta);
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
        let Some(selector) = self.patch_selector.as_mut() else {
            return;
        };
        if selector.filter_editing() {
            if selector.handle_filter_key(key) {
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
                selector.start_filter_edit();
                return;
            }
            _ => {}
        }
        let navigated = match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                selector.move_focus(-1);
                false
            }
            KeyCode::Right | KeyCode::Char('l') => {
                selector.move_focus(1);
                false
            }
            KeyCode::Up | KeyCode::Char('k') => {
                selector.move_focused_cursor(-1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selector.move_focused_cursor(1);
                true
            }
            KeyCode::PageUp => {
                selector.move_focused_page(-1);
                true
            }
            KeyCode::PageDown => {
                selector.move_focused_page(1);
                true
            }
            KeyCode::Home => {
                selector.move_focused_to_start();
                true
            }
            KeyCode::End => {
                selector.move_focused_to_end();
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
