use crossterm::event::KeyCode;

use super::super::{
    mml::build_cell_mml_from_data, DawApp, DawMode, DawPatchSelectPane, DawPlayState,
    FIRST_PLAYABLE_TRACK,
};

const PATCH_SELECT_PREVIEW_FALLBACK_PHRASE: &str = "c";

impl DawApp {
    fn move_patch_select_selection_by(&mut self, delta: isize) {
        let (items_len, cursor) = match self.patch_select_focus {
            DawPatchSelectPane::Patches => (self.patch_filtered.len(), &mut self.patch_cursor),
            DawPatchSelectPane::Favorites => (
                self.patch_favorite_items.len(),
                &mut self.patch_favorites_cursor,
            ),
        };
        if items_len == 0 {
            return;
        }
        let max_cursor = items_len.saturating_sub(1) as isize;
        let next_cursor = (*cursor as isize + delta).clamp(0, max_cursor) as usize;
        if next_cursor != *cursor {
            *cursor = next_cursor;
            self.preview_selected_patch();
        }
    }

    fn refresh_patch_select_favorites(&mut self) {
        let mut favorites = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (patch_name, _) in &self.patch_all {
            let is_favorite = self
                .patch_phrase_store
                .patches
                .get(patch_name)
                .is_some_and(|state| !state.favorites.is_empty());
            if is_favorite && seen.insert(patch_name.clone()) {
                favorites.push(patch_name.clone());
            }
        }

        let mut extra_favorites = self
            .patch_phrase_store
            .patches
            .iter()
            .filter_map(|(patch_name, state)| {
                (!state.favorites.is_empty() && seen.insert(patch_name.clone()))
                    .then_some(patch_name.clone())
            })
            .collect::<Vec<_>>();
        extra_favorites.sort();
        favorites.extend(extra_favorites);

        self.patch_favorite_items = favorites;
    }

    fn sync_patch_select_cursors(&mut self) {
        if self.patch_filtered.is_empty() {
            self.patch_cursor = 0;
        } else {
            self.patch_cursor = self.patch_cursor.min(self.patch_filtered.len() - 1);
        }

        if self.patch_favorite_items.is_empty() {
            self.patch_favorites_cursor = 0;
        } else {
            self.patch_favorites_cursor = self
                .patch_favorites_cursor
                .min(self.patch_favorite_items.len() - 1);
        }
    }

    fn patch_select_selected_patch_name(&self) -> Option<String> {
        match self.patch_select_focus {
            DawPatchSelectPane::Patches => self.patch_filtered.get(self.patch_cursor).cloned(),
            DawPatchSelectPane::Favorites => self
                .patch_favorite_items
                .get(self.patch_favorites_cursor)
                .cloned(),
        }
    }

    pub(in crate::daw) fn patch_select_favorite_items(&self) -> &[String] {
        &self.patch_favorite_items
    }

    fn patch_select_target_measure(&self) -> usize {
        self.cursor_measure.max(1).min(self.measures)
    }

    fn patch_select_preview_phrase(&self, target_measure: usize) -> String {
        match self.data[self.cursor_track][target_measure].trim() {
            "" => PATCH_SELECT_PREVIEW_FALLBACK_PHRASE.to_string(),
            phrase => phrase.to_string(),
        }
    }

    fn preview_selected_patch(&mut self) {
        if *self.play_state.lock().unwrap() == DawPlayState::Playing
            || self.cursor_track < FIRST_PLAYABLE_TRACK
        {
            return;
        }

        let Some(selected_patch_name) = self.patch_select_selected_patch_name() else {
            return;
        };
        let target_measure = self.patch_select_target_measure();
        let Some(measure_index) = target_measure.checked_sub(1) else {
            return;
        };

        let mut preview_data = vec![self.data[0].clone(), self.data[self.cursor_track].clone()];
        preview_data[1][0] = Self::build_patch_json(&selected_patch_name);
        preview_data[1][target_measure] = self.patch_select_preview_phrase(target_measure);

        let mut track_mmls = self.build_measure_track_mmls_for_measure(target_measure);
        track_mmls[self.cursor_track] =
            build_cell_mml_from_data(&preview_data, self.measures, 1, target_measure);

        if self.try_start_preview_with_track_mmls_for_test(measure_index, Some(track_mmls.clone()))
        {
            return;
        }

        self.start_preview_with_snapshot(measure_index, track_mmls, self.playback_track_gains());
    }

    fn update_patch_filter(&mut self) {
        self.patch_filtered = Self::filter_patch_names_by_query(&self.patch_all, &self.patch_query);
        self.patch_cursor = 0;
        self.sync_patch_select_cursors();
        self.preview_selected_patch();
    }

    fn cancel_patch_filter_input(&mut self) {
        self.patch_select_filter_active = false;
        if self.patch_query == self.patch_query_before_input {
            self.sync_patch_select_cursors();
            return;
        }

        self.patch_query = self.patch_query_before_input.clone();
        self.update_patch_filter();
    }

    pub(in crate::daw) fn start_patch_select_overlay(&mut self, initial_patch_name: Option<&str>) {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            self.append_log_line("音色選択は演奏トラックでのみ使用できます".to_string());
            return;
        }

        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            self.append_log_line("patches_dirs が設定されていません".to_string());
            return;
        }
        let Ok(patches) = crate::patches::collect_patch_pairs(&self.cfg) else {
            self.append_log_line("パッチの読み込みに失敗しました".to_string());
            return;
        };
        if patches.is_empty() {
            self.append_log_line("patches_dirs にパッチが見つかりません".to_string());
            return;
        }

        if crate::history::normalize_patch_phrase_store_for_available_patches(
            &mut self.patch_phrase_store,
            &patches,
        ) {
            self.mark_patch_phrase_store_dirty();
        }
        self.patch_all = patches;
        self.patch_query.clear();
        self.patch_query_before_input.clear();
        self.patch_filtered = self
            .patch_all
            .iter()
            .map(|(orig, _)| orig.clone())
            .collect();
        self.patch_cursor = initial_patch_name
            .map(|patch_name| {
                crate::patches::resolve_display_patch_name(&self.patch_all, patch_name)
                    .unwrap_or_else(|| patch_name.to_string())
            })
            .or_else(|| self.current_track_patch_name())
            .and_then(|patch_name| {
                self.patch_filtered
                    .iter()
                    .position(|patch| patch == &patch_name)
            })
            .unwrap_or(0);
        self.refresh_patch_select_favorites();
        self.patch_favorites_cursor = 0;
        self.patch_select_focus = DawPatchSelectPane::Patches;
        self.patch_select_filter_active = false;
        self.sync_patch_select_cursors();
        self.mode = DawMode::PatchSelect;
        self.preview_selected_patch();
    }

    pub(in crate::daw) fn handle_patch_select(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc if self.patch_select_filter_active => {
                self.cancel_patch_filter_input();
            }
            KeyCode::Esc => {
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('n') if !self.patch_select_filter_active => {
                self.start_history_overlay_for_patch_name(None);
            }
            KeyCode::Char('p') if !self.patch_select_filter_active => {
                let selected_patch_name = self.patch_select_selected_patch_name();
                let current_patch_name = self.current_track_patch_name();
                if let Some(patch_name) = selected_patch_name.or(current_patch_name) {
                    self.start_history_overlay_for_patch_name(Some(patch_name));
                } else {
                    self.append_log_line("patch name JSON が見つかりません".to_string());
                }
            }
            KeyCode::Char('t') if !self.patch_select_filter_active => {
                let selected_patch_name = self.patch_select_selected_patch_name();
                self.start_patch_select_overlay(selected_patch_name.as_deref());
            }
            KeyCode::Enter if self.patch_select_filter_active => {
                self.patch_select_filter_active = false;
                self.sync_patch_select_cursors();
            }
            KeyCode::Enter => {
                if let Some(selected_patch_name) = self.patch_select_selected_patch_name() {
                    let patch_json = Self::build_patch_json_with_filter_query(
                        &selected_patch_name,
                        Some(&self.patch_query),
                    );
                    if self.commit_insert_cell(self.cursor_track, 0, &patch_json) {
                        self.save();
                        self.sync_playback_mml_state();
                        if *self.play_state.lock().unwrap() == DawPlayState::Idle
                            && self.patch_select_target_measure() > 0
                            && self.entry_ptr != 0
                        {
                            self.start_preview(self.patch_select_target_measure() - 1);
                        }
                    }
                }
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('h') | KeyCode::Left if !self.patch_select_filter_active => {
                self.patch_select_focus = DawPatchSelectPane::Patches;
                self.sync_patch_select_cursors();
                self.preview_selected_patch();
            }
            KeyCode::Char('l') | KeyCode::Right if !self.patch_select_filter_active => {
                self.patch_select_focus = DawPatchSelectPane::Favorites;
                self.sync_patch_select_cursors();
                self.preview_selected_patch();
            }
            KeyCode::Char('j') | KeyCode::Down if !self.patch_select_filter_active => {
                self.move_patch_select_selection_by(1)
            }
            KeyCode::Char('k') | KeyCode::Up if !self.patch_select_filter_active => {
                self.move_patch_select_selection_by(-1)
            }
            KeyCode::Char('/') if !self.patch_select_filter_active => {
                self.patch_select_focus = DawPatchSelectPane::Patches;
                self.patch_query_before_input = self.patch_query.clone();
                self.patch_select_filter_active = true;
                self.sync_patch_select_cursors();
            }
            KeyCode::Backspace if self.patch_select_filter_active => {
                if self.patch_query.pop().is_some() {
                    self.update_patch_filter();
                }
            }
            KeyCode::Char('?') => self.enter_help(),
            KeyCode::Char(' ') if !self.patch_select_filter_active => {
                self.preview_selected_patch();
            }
            KeyCode::Char(c) if self.patch_select_filter_active => {
                self.patch_query.push(c);
                self.update_patch_filter();
            }
            _ => {}
        }
    }
}
