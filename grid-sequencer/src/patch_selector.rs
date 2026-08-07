//! PATCH 欄から開く、mouse/keyboard 共用の行単位 patch selector。

use std::ops::Range;

use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::{
    patches::{group_patch_pairs_by_category, PatchCategory},
    random::random_index,
};
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::state::patch_is_chord_candidate;
use crate::{GridPatchLoad, GridSequencerContext, GridSequencerScreen, CHORD_ROW};

mod layout;

use layout::contains;
pub(crate) use layout::PatchSelectorLayout;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PatchSelector {
    pub(crate) instance: usize,
    pub(crate) categories: Vec<PatchCategory>,
    pub(crate) category_cursor: usize,
    pub(crate) patch_cursor: usize,
    poly_only: bool,
    original_patch: Option<String>,
    previewed_patch: Option<String>,
}

impl PatchSelector {
    fn new(
        instance: usize,
        current_patch: Option<&str>,
        ctx: &GridSequencerContext<'_>,
        poly_only: bool,
    ) -> Option<Self> {
        if !ctx.patch_dirs_configured {
            return None;
        }
        let GridPatchLoad::Ready(pairs) = &ctx.patch_load else {
            return None;
        };
        let pairs = pairs
            .iter()
            .filter(|(patch, _)| {
                !poly_only || ctx.voicing.cached_voicing(patch) == Some(PatchVoicing::Poly)
            })
            .cloned()
            .collect::<Vec<_>>();
        let categories = group_patch_pairs_by_category(&pairs);
        if categories.is_empty() {
            return None;
        }
        let selected = current_patch.and_then(|current| {
            categories
                .iter()
                .enumerate()
                .find_map(|(category_index, category)| {
                    category
                        .patches
                        .iter()
                        .position(|patch| patch == current)
                        .map(|patch_index| (category_index, patch_index))
                })
        });
        Some(Self {
            instance,
            categories,
            category_cursor: selected.map_or(0, |(category, _)| category),
            patch_cursor: selected.map_or(0, |(_, patch)| patch),
            poly_only,
            original_patch: current_patch.map(str::to_string),
            previewed_patch: current_patch.map(str::to_string),
        })
    }

    pub(crate) fn selected_category(&self) -> &PatchCategory {
        &self.categories[self.category_cursor]
    }

    pub(crate) fn selected_patch(&self) -> Option<&str> {
        self.selected_category()
            .patches
            .get(self.patch_cursor)
            .map(String::as_str)
    }

    fn move_category(&mut self, delta: isize) {
        let next = move_cursor(self.category_cursor, delta, self.categories.len());
        if next != self.category_cursor {
            self.category_cursor = next;
            self.patch_cursor = 0;
        }
    }

    fn move_patch(&mut self, delta: isize) {
        self.patch_cursor = move_cursor(
            self.patch_cursor,
            delta,
            self.selected_category().patches.len(),
        );
    }

    fn select_category(&mut self, index: usize) {
        if index < self.categories.len() && index != self.category_cursor {
            self.category_cursor = index;
            self.patch_cursor = 0;
        }
    }

    fn select_patch(&mut self, index: usize) {
        if index < self.selected_category().patches.len() {
            self.patch_cursor = index;
        }
    }

    fn select_random_patch(&mut self) {
        let total = self
            .categories
            .iter()
            .map(|category| category.patches.len())
            .sum::<usize>();
        if total <= 1 {
            return;
        }
        let current = self.categories[..self.category_cursor]
            .iter()
            .map(|category| category.patches.len())
            .sum::<usize>()
            + self.patch_cursor;
        let Some(random) = random_index(total - 1) else {
            return;
        };
        let target = if random >= current {
            random + 1
        } else {
            random
        };
        let mut offset = 0;
        for (category, item) in self.categories.iter().enumerate() {
            let next = offset + item.patches.len();
            if target < next {
                self.category_cursor = category;
                self.patch_cursor = target - offset;
                return;
            }
            offset = next;
        }
    }

    pub(crate) fn category_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        visible_range(
            self.categories.len(),
            self.category_cursor,
            usize::from(layout.category_list.height),
        )
    }

    pub(crate) fn patch_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        visible_range(
            self.selected_category().patches.len(),
            self.patch_cursor,
            usize::from(layout.patch_list.height),
        )
    }
}

impl GridSequencerScreen {
    /// PATCH名上のwheelで、そのinstanceの用途に合う別patchをランダム適用する。
    pub(crate) fn randomize_instance_patch(
        &mut self,
        instance: usize,
        ctx: &GridSequencerContext<'_>,
    ) {
        let Some(item) = self.state.instances().get(instance) else {
            return;
        };
        let current = item.patch.as_deref();
        let chord_target = instance == CHORD_ROW && self.state.chord().is_some();
        let candidates = ctx
            .patches()
            .iter()
            .filter(|(display, lower)| {
                patch_is_chord_candidate(display, lower, ctx.voicing, ctx.chord_patch_categories)
                    == chord_target
                    && current != Some(display.as_str())
            })
            .map(|(display, _)| display)
            .collect::<Vec<_>>();
        let Some(index) = random_index(candidates.len()) else {
            return;
        };
        let patch = candidates[index].clone();
        let undo = self.capture_undo();
        if self.state.set_instance_patch(instance, patch.clone()) {
            self.begin_manual_edit();
            self.prepare_patch(instance, Some(&patch), "wheel-random");
            self.commit_undo(undo);
        }
    }

    pub(crate) fn open_patch_selector(&mut self, instance: usize, ctx: &GridSequencerContext<'_>) {
        let Some(current) = self
            .state
            .instances()
            .get(instance)
            .map(|item| item.patch.as_deref())
        else {
            return;
        };
        let poly_only = instance == CHORD_ROW && self.state.chord().is_some();
        self.patch_selector = PatchSelector::new(instance, current, ctx, poly_only);
    }

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

    fn preview_patch_selection(&mut self, ctx: &GridSequencerContext<'_>) {
        let Some((instance, patch, poly_only, already_previewed)) =
            self.patch_selector.as_ref().and_then(|selector| {
                selector.selected_patch().map(|patch| {
                    (
                        selector.instance,
                        patch.to_string(),
                        selector.poly_only,
                        selector.previewed_patch.as_deref() == Some(patch),
                    )
                })
            })
        else {
            return;
        };
        if already_previewed || !patch_is_available(&patch, poly_only, ctx) {
            return;
        }
        self.prepare_patch(instance, Some(&patch), "preview");
        let Some(selector) = self.patch_selector.as_mut() else {
            return;
        };
        if selector.instance == instance && selector.selected_patch() == Some(patch.as_str()) {
            selector.previewed_patch = Some(patch);
        }
    }

    pub(crate) fn cancel_patch_selector(&mut self) {
        let Some(selector) = self.patch_selector.take() else {
            return;
        };
        if selector.previewed_patch != selector.original_patch {
            self.prepare_patch(
                selector.instance,
                selector.original_patch.as_deref(),
                "cancel-restore",
            );
        }
    }

    fn apply_patch_selection(&mut self, ctx: &GridSequencerContext<'_>) {
        let Some(selector) = self.patch_selector.take() else {
            return;
        };
        let Some(patch) = selector.selected_patch().map(str::to_string) else {
            return;
        };
        let instance = selector.instance;
        if !patch_is_available(&patch, selector.poly_only, ctx) {
            if selector.previewed_patch != selector.original_patch {
                self.prepare_patch(
                    instance,
                    selector.original_patch.as_deref(),
                    "invalid-restore",
                );
            }
            return;
        }
        let undo = self.capture_undo();
        if self.state.set_instance_patch(instance, patch.clone()) {
            self.begin_manual_edit();
            // preview の送信先 bank は、確定までの間に切り替わり得る。patch 名が同じ
            // というだけで省略せず、HOLD へ固定した後の active instance へ必ず積む。
            self.prepare_patch(instance, Some(&patch), "confirm");
            self.commit_undo(undo);
        }
    }

    pub(crate) fn prepare_instance_patch(&self, instance: usize) {
        let Some(patch) = self
            .state
            .instances()
            .get(instance)
            .map(|item| item.patch.as_deref())
        else {
            return;
        };
        self.prepare_patch(instance, patch, "undo");
    }

    fn prepare_patch(&self, instance: usize, patch: Option<&str>, reason: &'static str) {
        if self.state.instances().get(instance).is_none() {
            return;
        }
        if let Some(sender) = &self.midi_sender {
            let instance_id = self.state.instance_id(instance);
            let request_id = sender.set_row_patch(instance, instance_id, patch, reason);
            crate::log_line(&format!(
                "grid-sequencer: patch-selector request={request_id} reason={reason} instance={} \
                 active_bank={} instance={instance_id} chord_index={} patch={patch:?}",
                instance + 1,
                self.state.bank(),
                self.state
                    .chord()
                    .map_or("-".to_string(), |chord| chord.index().to_string()),
            ));
        }
    }
}

fn patch_is_available(patch: &str, poly_only: bool, ctx: &GridSequencerContext<'_>) -> bool {
    ctx.patch_dirs_configured
        && ctx.patches().iter().any(|(display, _)| display == patch)
        && (!poly_only || ctx.voicing.cached_voicing(patch) == Some(PatchVoicing::Poly))
}

fn move_cursor(current: usize, delta: isize, len: usize) -> usize {
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

fn visible_range(total: usize, cursor: usize, height: usize) -> Range<usize> {
    let visible = height.min(total);
    let start = cursor
        .saturating_sub(visible / 2)
        .min(total.saturating_sub(visible));
    start..start + visible
}

#[cfg(test)]
mod tests;
