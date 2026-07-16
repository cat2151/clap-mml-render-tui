use ratatui::widgets::ListState;

use crate::patches::PatchCategory;

use super::super::{PatchLoadState, TuiApp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) enum KeyboardPatchCatalogStatus {
    Loading,
    NotConfigured,
    Ready,
    Error(String),
}

pub(in crate::tui) struct KeyboardPatchCatalog {
    status: KeyboardPatchCatalogStatus,
    categories: Vec<PatchCategory>,
    category_cursor: Option<usize>,
    patch_cursor: Option<usize>,
    category_list_state: ListState,
    patch_list_state: ListState,
}

impl Default for KeyboardPatchCatalog {
    fn default() -> Self {
        Self {
            status: KeyboardPatchCatalogStatus::Loading,
            categories: Vec::new(),
            category_cursor: None,
            patch_cursor: None,
            category_list_state: ListState::default(),
            patch_list_state: ListState::default(),
        }
    }
}

impl KeyboardPatchCatalog {
    pub(in crate::tui) fn status(&self) -> &KeyboardPatchCatalogStatus {
        &self.status
    }

    pub(super) fn is_ready(&self) -> bool {
        self.status == KeyboardPatchCatalogStatus::Ready
    }

    pub(super) fn set_loading(&mut self) {
        self.status = KeyboardPatchCatalogStatus::Loading;
    }

    pub(super) fn set_not_configured(&mut self) {
        self.clear_with_status(KeyboardPatchCatalogStatus::NotConfigured);
    }

    pub(super) fn set_error(&mut self, error: String) {
        self.clear_with_status(KeyboardPatchCatalogStatus::Error(error));
    }

    fn clear_with_status(&mut self, status: KeyboardPatchCatalogStatus) {
        self.status = status;
        self.categories.clear();
        self.category_cursor = None;
        self.patch_cursor = None;
        self.category_list_state.select(None);
        self.patch_list_state.select(None);
    }

    pub(super) fn load(&mut self, categories: Vec<PatchCategory>, current_patch: Option<&str>) {
        self.status = KeyboardPatchCatalogStatus::Ready;
        self.categories = categories;
        let selection = current_patch.and_then(|current_patch| {
            self.categories
                .iter()
                .enumerate()
                .find_map(|(category_index, category)| {
                    category
                        .patches
                        .iter()
                        .position(|patch| patch == current_patch)
                        .map(|patch_index| (category_index, patch_index))
                })
        });
        self.category_cursor = selection.map(|(category, _)| category);
        self.patch_cursor = selection.map(|(_, patch)| patch);
        self.sync_list_states(1, 1);
    }

    pub(in crate::tui) fn categories(&self) -> &[PatchCategory] {
        &self.categories
    }

    pub(in crate::tui) fn selected_category_index(&self) -> Option<usize> {
        self.category_cursor
    }

    pub(in crate::tui) fn selected_patch_index(&self) -> Option<usize> {
        self.patch_cursor
    }

    pub(in crate::tui) fn selected_category(&self) -> Option<&PatchCategory> {
        self.category_cursor
            .and_then(|index| self.categories.get(index))
    }

    pub(super) fn move_patch_by(&mut self, delta: isize) -> Option<String> {
        if self.categories.is_empty() {
            return None;
        }
        let Some(category_index) = self.category_cursor else {
            return self.select(0, 0);
        };
        let patches_len = self.categories[category_index].patches.len();
        if patches_len == 0 {
            return None;
        }
        let Some(patch_index) = self.patch_cursor else {
            return self.select(category_index, 0);
        };
        let next = (patch_index as isize + delta).clamp(0, patches_len.saturating_sub(1) as isize)
            as usize;
        if next == patch_index {
            return None;
        }
        self.select(category_index, next)
    }

    pub(super) fn move_category_by(&mut self, delta: isize) -> Option<String> {
        if self.categories.is_empty() {
            return None;
        }
        let Some(category_index) = self.category_cursor else {
            return self.select(0, 0);
        };
        let next = (category_index as isize + delta)
            .clamp(0, self.categories.len().saturating_sub(1) as isize) as usize;
        if next == category_index {
            return None;
        }
        self.select(next, 0)
    }

    fn select(&mut self, category_index: usize, patch_index: usize) -> Option<String> {
        let patch = self
            .categories
            .get(category_index)?
            .patches
            .get(patch_index)?
            .clone();
        self.category_cursor = Some(category_index);
        self.patch_cursor = Some(patch_index);
        self.category_list_state.select(Some(category_index));
        self.patch_list_state.select(Some(patch_index));
        Some(patch)
    }

    pub(in crate::tui) fn sync_list_states(
        &mut self,
        category_page_size: usize,
        patch_page_size: usize,
    ) {
        self.category_list_state.select(self.category_cursor);
        sync_list_offset(
            &mut self.category_list_state,
            self.category_cursor,
            self.categories.len(),
            category_page_size,
        );

        let patch_count = self
            .selected_category()
            .map(|category| category.patches.len())
            .unwrap_or(0);
        self.patch_list_state.select(self.patch_cursor);
        sync_list_offset(
            &mut self.patch_list_state,
            self.patch_cursor,
            patch_count,
            patch_page_size,
        );
    }

    pub(in crate::tui) fn category_list_state_mut(&mut self) -> &mut ListState {
        &mut self.category_list_state
    }

    pub(in crate::tui) fn patch_list_state_mut(&mut self) -> &mut ListState {
        &mut self.patch_list_state
    }
}

fn sync_list_offset(
    state: &mut ListState,
    cursor: Option<usize>,
    item_count: usize,
    page_size: usize,
) {
    let Some(cursor) = cursor else {
        *state.offset_mut() = 0;
        return;
    };
    let visible = page_size.max(1).min(item_count.max(1));
    let max_offset = item_count.saturating_sub(visible);
    let current = state.offset().min(max_offset);
    let next = if cursor < current {
        cursor
    } else if cursor >= current.saturating_add(visible) {
        cursor.saturating_add(1).saturating_sub(visible)
    } else {
        current
    };
    *state.offset_mut() = next.min(max_offset);
}

impl<'a> TuiApp<'a> {
    pub(in crate::tui) fn sync_keyboard_patch_catalog(&mut self) {
        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            self.keyboard_state.patch_catalog.set_not_configured();
            return;
        }
        if self.keyboard_state.patch_catalog.is_ready() {
            return;
        }

        enum LoadedCatalog {
            Loading,
            Ready(Vec<(String, String)>),
            Error(String),
        }
        let loaded = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => LoadedCatalog::Loading,
                PatchLoadState::Ready(pairs) => LoadedCatalog::Ready(pairs.clone()),
                PatchLoadState::Err(error) => LoadedCatalog::Error(error.clone()),
            }
        };

        match loaded {
            LoadedCatalog::Loading => self.keyboard_state.patch_catalog.set_loading(),
            LoadedCatalog::Error(error) => self.keyboard_state.patch_catalog.set_error(error),
            LoadedCatalog::Ready(pairs) => {
                let current_patch = self
                    .keyboard_state
                    .patch()
                    .and_then(|patch| crate::patches::resolve_display_patch_name(&pairs, patch));
                let categories = crate::patches::group_patch_pairs_by_category(&pairs);
                self.keyboard_state
                    .patch_catalog
                    .load(categories, current_patch.as_deref());
            }
        }
    }

    pub(super) fn move_keyboard_patch_by(&mut self, delta: isize) {
        self.sync_keyboard_patch_catalog();
        let selected = self.keyboard_state.patch_catalog.move_patch_by(delta);
        self.apply_keyboard_patch_selection(selected);
    }

    pub(super) fn move_keyboard_patch_category_by(&mut self, delta: isize) {
        self.sync_keyboard_patch_catalog();
        let selected = self.keyboard_state.patch_catalog.move_category_by(delta);
        self.apply_keyboard_patch_selection(selected);
    }

    fn apply_keyboard_patch_selection(&mut self, selected: Option<String>) {
        let Some(patch) = selected else {
            return;
        };
        let previous_patch = self.keyboard_state.patch().map(str::to_string);
        if previous_patch.as_deref() == Some(patch.as_str()) {
            return;
        }
        let note_offs = self.keyboard_state.take_reset_messages();
        self.keyboard_state.patch = Some(patch.clone());
        if let Some(sender) = &self.keyboard_midi_sender {
            sender.set_patch(note_offs, previous_patch.as_deref(), Some(&patch));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn categories() -> Vec<PatchCategory> {
        vec![
            PatchCategory {
                name: "Lead".to_string(),
                patches: vec!["Lead 1".to_string(), "Lead 2".to_string()],
            },
            PatchCategory {
                name: "Pad".to_string(),
                patches: (0..12).map(|index| format!("Pad {index}")).collect(),
            },
        ]
    }

    #[test]
    fn load_selects_the_current_patch_without_changing_unknown_patch() {
        let mut catalog = KeyboardPatchCatalog::default();
        catalog.load(categories(), Some("Lead 2"));
        assert_eq!(catalog.selected_category_index(), Some(0));
        assert_eq!(catalog.selected_patch_index(), Some(1));

        catalog.load(categories(), Some("Unknown"));
        assert_eq!(catalog.selected_category_index(), None);
        assert_eq!(catalog.selected_patch_index(), None);
    }

    #[test]
    fn patch_navigation_clamps_and_moves_by_ten() {
        let mut catalog = KeyboardPatchCatalog::default();
        catalog.load(categories(), Some("Pad 0"));

        assert_eq!(catalog.move_patch_by(10).as_deref(), Some("Pad 10"));
        assert_eq!(catalog.move_patch_by(10).as_deref(), Some("Pad 11"));
        assert_eq!(catalog.move_patch_by(1), None);
        assert_eq!(catalog.move_patch_by(-10).as_deref(), Some("Pad 1"));
    }

    #[test]
    fn category_navigation_selects_the_first_patch_and_clamps() {
        let mut catalog = KeyboardPatchCatalog::default();
        catalog.load(categories(), Some("Lead 2"));

        assert_eq!(catalog.move_category_by(1).as_deref(), Some("Pad 0"));
        assert_eq!(catalog.move_category_by(1), None);
        assert_eq!(catalog.move_category_by(-1).as_deref(), Some("Lead 1"));
        assert_eq!(catalog.move_category_by(-1), None);
    }

    #[test]
    fn first_navigation_from_an_unknown_patch_selects_the_first_patch() {
        let mut catalog = KeyboardPatchCatalog::default();
        catalog.load(categories(), Some("Unknown"));

        assert_eq!(catalog.move_patch_by(-1).as_deref(), Some("Lead 1"));
        assert_eq!(catalog.selected_category_index(), Some(0));
        assert_eq!(catalog.selected_patch_index(), Some(0));
    }
}
