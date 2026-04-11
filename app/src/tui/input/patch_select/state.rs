use mmlabc_to_smf::mml_preprocessor;
use ratatui::widgets::ListState;

use crate::tui::{Mode, PatchLoadState, PatchSelectPane, TuiApp};

use super::PATCH_SELECT_PREVIEW_FALLBACK_PHRASE;

impl<'a> TuiApp<'a> {
    fn sort_patch_select_pairs(&mut self) {
        crate::patches::sort_patch_pairs(&mut self.patch_all, self.patch_select_sort_order);
    }

    fn ensure_patch_select_source_order(&mut self) {
        if self.patch_all_source_order.is_empty() {
            self.patch_all_source_order = self.patch_all.clone();
        }
    }

    fn move_patch_cursor_by(&mut self, delta: isize) {
        if self.patch_filtered.is_empty() {
            return;
        }
        let max_cursor = self.patch_filtered.len().saturating_sub(1) as isize;
        let next_cursor = (self.patch_cursor as isize + delta).clamp(0, max_cursor) as usize;
        if next_cursor != self.patch_cursor {
            self.patch_cursor = next_cursor;
            self.patch_list_state.select(Some(self.patch_cursor));
            Self::sync_overlay_list_offset(
                &mut self.patch_list_state,
                self.patch_cursor,
                self.patch_filtered.len(),
                self.patch_select_page_size,
            );
            self.preview_selected_patch();
        }
    }

    fn move_patch_favorites_cursor_by(&mut self, delta: isize) {
        if self.patch_favorite_items.is_empty() {
            return;
        }
        let max_cursor = self.patch_favorite_items.len().saturating_sub(1) as isize;
        let next_cursor =
            (self.patch_favorites_cursor as isize + delta).clamp(0, max_cursor) as usize;
        if next_cursor != self.patch_favorites_cursor {
            self.patch_favorites_cursor = next_cursor;
            self.patch_favorites_state
                .select(Some(self.patch_favorites_cursor));
            Self::sync_overlay_list_offset(
                &mut self.patch_favorites_state,
                self.patch_favorites_cursor,
                self.patch_favorite_items.len(),
                self.patch_select_page_size,
            );
            self.preview_selected_patch();
        }
    }

    pub(super) fn move_patch_select_selection_by(&mut self, delta: isize) {
        match self.patch_select_focus {
            PatchSelectPane::Patches => self.move_patch_cursor_by(delta),
            PatchSelectPane::Favorites => self.move_patch_favorites_cursor_by(delta),
        }
    }

    pub(super) fn start_patch_select_with_initial_patch_name(
        &mut self,
        initial_patch_name: Option<&str>,
    ) {
        {
            let state = self.patch_load_state.lock().unwrap();
            if let PatchLoadState::Ready(pairs) = &*state {
                self.patch_all_source_order = pairs.clone();
            }
        }
        self.patch_all = self.patch_all_source_order.clone();
        if self.patch_select_sort_order == crate::patches::PatchSortOrder::Category {
            self.sort_patch_select_pairs();
        }
        if crate::history::normalize_patch_phrase_store_for_available_patches(
            &mut self.patch_phrase_store,
            &self.patch_all,
        ) {
            self.patch_phrase_store_dirty = true;
        }
        self.patch_query = String::new();
        self.patch_query_textarea = crate::text_input::new_single_line_textarea("");
        self.patch_filtered = self
            .patch_all
            .iter()
            .map(|(orig, _)| orig.clone())
            .collect();
        self.patch_select_focus = PatchSelectPane::Patches;
        self.patch_select_filter_active = false;
        self.patch_cursor = initial_patch_name
            .map(|patch_name| {
                self.resolve_loaded_patch_name(patch_name)
                    .unwrap_or_else(|| patch_name.to_string())
            })
            .or_else(|| self.current_line_patch_name())
            .and_then(|patch_name| {
                self.patch_filtered
                    .iter()
                    .position(|patch| patch == &patch_name)
            })
            .unwrap_or(0);
        self.refresh_patch_select_favorites();
        self.patch_favorites_cursor = 0;
        self.patch_list_state = ListState::default();
        self.patch_favorites_state = ListState::default();
        self.sync_patch_select_states();
        self.mode = Mode::PatchSelect;
        Self::log_notepad_event(format!(
            "tone-select open patches={} favorites={} focus={:?} cursor={} selected={:?}",
            self.patch_filtered.len(),
            self.patch_favorite_items.len(),
            self.patch_select_focus,
            self.patch_cursor,
            self.patch_select_selected_patch_name()
        ));
    }

    pub(super) fn patch_select_current_phrase(&self) -> Option<String> {
        let line = self.lines.get(self.cursor)?;
        let preprocessed = mml_preprocessor::extract_embedded_json(line);
        Some(match preprocessed.remaining_mml.trim() {
            "" => PATCH_SELECT_PREVIEW_FALLBACK_PHRASE.to_string(),
            remaining => remaining.to_string(),
        })
    }

    fn rebuild_patch_select_favorite_items(&self) -> Vec<String> {
        let mut favorites = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for patch_name in &self.patch_phrase_store.favorite_patches {
            let is_favorite = self
                .patch_phrase_store
                .patches
                .get(patch_name)
                .is_some_and(|state| !state.favorites.is_empty());
            if is_favorite && seen.insert(patch_name.clone()) {
                favorites.push(patch_name.clone());
            }
        }

        favorites
    }

    pub(super) fn refresh_patch_select_favorites(&mut self) {
        let patch_order = self
            .patch_all
            .iter()
            .map(|(patch_name, _)| patch_name.clone())
            .collect::<Vec<_>>();
        if crate::history::sync_patch_favorite_order(&mut self.patch_phrase_store, &patch_order) {
            self.patch_phrase_store_dirty = true;
        }
        self.patch_favorite_items = self.rebuild_patch_select_favorite_items();
    }

    pub(super) fn toggle_patch_select_sort_order(&mut self) {
        self.ensure_patch_select_source_order();
        let selected_patch = self.patch_filtered.get(self.patch_cursor).cloned();
        let selected_favorite = self
            .patch_favorite_items
            .get(self.patch_favorites_cursor)
            .cloned();

        self.patch_select_sort_order = self.patch_select_sort_order.toggle();
        self.patch_all = self.patch_all_source_order.clone();
        if self.patch_select_sort_order == crate::patches::PatchSortOrder::Category {
            self.sort_patch_select_pairs();
        }
        self.patch_filtered = crate::tui::filter_patches(&self.patch_all, &self.patch_query);
        self.refresh_patch_select_favorites();

        self.patch_cursor = selected_patch
            .and_then(|patch_name| {
                self.patch_filtered
                    .iter()
                    .position(|patch| patch == &patch_name)
            })
            .unwrap_or(0);
        self.patch_favorites_cursor = selected_favorite
            .and_then(|patch_name| {
                self.patch_favorite_items
                    .iter()
                    .position(|patch| patch == &patch_name)
            })
            .unwrap_or(0);

        self.sync_patch_select_states();
        self.preview_selected_patch();
    }

    pub(in crate::tui) fn patch_select_favorite_items(&self) -> &[String] {
        &self.patch_favorite_items
    }

    pub(super) fn sync_patch_select_states(&mut self) {
        if self.patch_filtered.is_empty() {
            self.patch_cursor = 0;
            self.patch_list_state.select(None);
        } else {
            self.patch_cursor = self.patch_cursor.min(self.patch_filtered.len() - 1);
            self.patch_list_state.select(Some(self.patch_cursor));
            Self::sync_overlay_list_offset(
                &mut self.patch_list_state,
                self.patch_cursor,
                self.patch_filtered.len(),
                self.patch_select_page_size,
            );
        }

        let favorites_len = self.patch_favorite_items.len();
        if favorites_len == 0 {
            self.patch_favorites_cursor = 0;
            self.patch_favorites_state.select(None);
        } else {
            self.patch_favorites_cursor = self.patch_favorites_cursor.min(favorites_len - 1);
            self.patch_favorites_state
                .select(Some(self.patch_favorites_cursor));
            Self::sync_overlay_list_offset(
                &mut self.patch_favorites_state,
                self.patch_favorites_cursor,
                favorites_len,
                self.patch_select_page_size,
            );
        }
    }

    fn patch_select_patch_name_for_selection(
        &self,
        focus: PatchSelectPane,
        cursor: usize,
    ) -> Option<String> {
        match focus {
            PatchSelectPane::Patches => self.patch_filtered.get(cursor).cloned(),
            PatchSelectPane::Favorites => self.patch_favorite_items.get(cursor).cloned(),
        }
    }

    pub(super) fn patch_select_selected_patch_name(&self) -> Option<String> {
        let cursor = match self.patch_select_focus {
            PatchSelectPane::Patches => self.patch_cursor,
            PatchSelectPane::Favorites => self.patch_favorites_cursor,
        };
        self.patch_select_patch_name_for_selection(self.patch_select_focus, cursor)
    }

    pub(super) fn patch_select_preview_mml_for_selection(
        &self,
        focus: PatchSelectPane,
        cursor: usize,
    ) -> Option<String> {
        let patch_name = self.patch_select_patch_name_for_selection(focus, cursor)?;
        let phrase = self.patch_select_current_phrase()?;
        let json = Self::build_patch_json(&patch_name);
        Some(format!("{json} {phrase}"))
    }

    pub(super) fn patch_select_preview_mml(&self) -> Option<String> {
        let cursor = match self.patch_select_focus {
            PatchSelectPane::Patches => self.patch_cursor,
            PatchSelectPane::Favorites => self.patch_favorites_cursor,
        };
        self.patch_select_preview_mml_for_selection(self.patch_select_focus, cursor)
    }
}
