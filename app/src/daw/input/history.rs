use crossterm::event::KeyCode;

use super::super::DawHistoryPane;
use super::super::{
    mml::build_cell_mml_from_data, DawApp, DawMode, DawPlayState, FIRST_PLAYABLE_TRACK,
};

impl DawApp {
    pub(in crate::daw) fn start_history_overlay_for_patch_name(
        &mut self,
        patch_name: Option<String>,
    ) {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            return;
        }
        self.history_overlay_patch_name = patch_name;
        self.history_overlay_query.clear();
        self.history_overlay_focus = DawHistoryPane::History;
        self.history_overlay_history_cursor = 0;
        self.history_overlay_favorites_cursor = 0;
        self.history_overlay_filter_active = false;
        self.sync_history_overlay_cursors();
        self.mode = DawMode::History;
    }

    pub(in crate::daw) fn history_overlay_history_items(&self) -> Vec<String> {
        let items = if let Some(patch_name) = self.history_overlay_patch_name.as_deref() {
            self.patch_phrase_store
                .patches
                .get(patch_name)
                .map(|state| state.history.clone())
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| vec!["c".to_string()])
        } else {
            self.patch_phrase_store
                .notepad
                .history
                .iter()
                .filter(|item| Self::extract_patch_phrase(item).is_some())
                .cloned()
                .collect()
        };
        crate::tui::filter_items(&items, &self.history_overlay_query)
    }

    pub(in crate::daw) fn history_overlay_favorite_items(&self) -> Vec<String> {
        let items = if let Some(patch_name) = self.history_overlay_patch_name.as_deref() {
            self.patch_phrase_store
                .patches
                .get(patch_name)
                .map(|state| state.favorites.clone())
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| vec!["c".to_string()])
        } else {
            self.patch_phrase_store
                .notepad
                .favorites
                .iter()
                .filter(|item| Self::extract_patch_phrase(item).is_some())
                .cloned()
                .collect()
        };
        crate::tui::filter_items(&items, &self.history_overlay_query)
    }

    fn sync_history_overlay_cursors(&mut self) {
        let history_len = self.history_overlay_history_items().len();
        if history_len == 0 {
            self.history_overlay_history_cursor = 0;
        } else {
            self.history_overlay_history_cursor =
                self.history_overlay_history_cursor.min(history_len - 1);
        }

        let favorites_len = self.history_overlay_favorite_items().len();
        if favorites_len == 0 {
            self.history_overlay_favorites_cursor = 0;
        } else {
            self.history_overlay_favorites_cursor =
                self.history_overlay_favorites_cursor.min(favorites_len - 1);
        }
    }

    pub(in crate::daw) fn start_history_overlay(&mut self) {
        self.start_history_overlay_for_patch_name(self.current_track_patch_name());
    }

    fn selected_history_overlay_item(&self) -> Option<String> {
        match self.history_overlay_focus {
            DawHistoryPane::History => self
                .history_overlay_history_items()
                .get(self.history_overlay_history_cursor)
                .cloned(),
            DawHistoryPane::Favorites => self
                .history_overlay_favorite_items()
                .get(self.history_overlay_favorites_cursor)
                .cloned(),
        }
    }

    fn history_overlay_target_measure(&self) -> usize {
        self.cursor_measure.max(1).min(self.measures)
    }

    fn preview_selected_history_overlay_item(&mut self) {
        if *self.play_state.lock().unwrap() == DawPlayState::Playing {
            return;
        }

        let Some(selected) = self.selected_history_overlay_item() else {
            return;
        };
        let target_measure = self.history_overlay_target_measure();
        let Some(measure_index) = target_measure.checked_sub(1) else {
            return;
        };

        let mut preview_data = vec![self.data[0].clone(), self.data[self.cursor_track].clone()];
        match self.history_overlay_patch_name.as_deref() {
            Some(_) => {
                preview_data[1][target_measure] = selected;
            }
            None => {
                let Some((patch_name, phrase)) = Self::extract_patch_phrase(&selected) else {
                    return;
                };
                preview_data[1][0] = Self::build_patch_json(&patch_name);
                preview_data[1][target_measure] = phrase;
            }
        }

        let mut track_mmls = self.build_measure_track_mmls_for_measure(target_measure);
        track_mmls[self.cursor_track] =
            build_cell_mml_from_data(&preview_data, self.measures, 1, target_measure);

        if self.try_start_preview_with_track_mmls_for_test(measure_index, Some(track_mmls.clone()))
        {
            return;
        }

        self.start_preview_with_snapshot(measure_index, track_mmls, self.playback_track_gains());
    }

    fn apply_history_overlay_selection(&mut self, selected: String) {
        let target_measure = self.history_overlay_target_measure();
        if self.cursor_measure == 0 {
            self.cursor_measure = target_measure;
            self.update_ab_repeat_follow_end_with_cursor();
        }

        match self.history_overlay_patch_name.clone() {
            Some(patch_name) => {
                let previous = self.data[self.cursor_track][target_measure]
                    .trim()
                    .to_string();
                if !previous.is_empty() {
                    let state = self
                        .patch_phrase_store
                        .patches
                        .entry(patch_name)
                        .or_default();
                    Self::push_front_dedup(&mut state.history, previous);
                }

                if self.commit_insert_cell(self.cursor_track, target_measure, &selected) {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
            None => {
                let Some((patch_name, phrase)) = Self::extract_patch_phrase(&selected) else {
                    return;
                };
                let patch_json = Self::build_patch_json(&patch_name);
                let previous = self.data[self.cursor_track][target_measure]
                    .trim()
                    .to_string();
                if !previous.is_empty() {
                    Self::push_front_dedup(
                        &mut self.patch_phrase_store.notepad.history,
                        format!("{patch_json} {previous}"),
                    );
                }

                let init_changed = self.commit_insert_cell(self.cursor_track, 0, &patch_json);
                let phrase_changed =
                    self.commit_insert_cell(self.cursor_track, target_measure, &phrase);
                if init_changed || phrase_changed {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
        }

        self.mark_patch_phrase_store_dirty();
        if *self.play_state.lock().unwrap() == DawPlayState::Idle
            && target_measure > 0
            && self.entry_ptr != 0
        {
            self.start_preview(target_measure - 1);
        }
        self.mode = DawMode::Normal;
    }

    pub(in crate::daw) fn handle_history_overlay(&mut self, key: KeyCode) {
        if self.history_overlay_filter_active {
            match key {
                KeyCode::Esc => {
                    self.history_overlay_filter_active = false;
                    self.mode = DawMode::Normal;
                }
                KeyCode::Enter => {
                    self.history_overlay_filter_active = false;
                    self.sync_history_overlay_cursors();
                }
                KeyCode::Backspace => {
                    self.history_overlay_query.pop();
                    self.sync_history_overlay_cursors();
                    self.preview_selected_history_overlay_item();
                    if self.history_overlay_query.is_empty() {
                        self.history_overlay_filter_active = false;
                    }
                }
                KeyCode::Char('?') => self.enter_help(),
                KeyCode::Char(c) => {
                    self.history_overlay_query.push(c);
                    self.sync_history_overlay_cursors();
                    self.preview_selected_history_overlay_item();
                }
                _ => {}
            }
            return;
        }

        let history_len = self.history_overlay_history_items().len();
        let favorites_len = self.history_overlay_favorite_items().len();

        match key {
            KeyCode::Esc => {
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('n') => {
                self.start_history_overlay_for_patch_name(None);
            }
            KeyCode::Char('p') => {
                let selected_patch_name = self.selected_history_overlay_item().and_then(|mml| {
                    Self::extract_patch_phrase(&mml).map(|(patch_name, _)| patch_name)
                });
                let current_patch_name = self.current_track_patch_name();
                if let Some(patch_name) = selected_patch_name.or(current_patch_name) {
                    self.start_history_overlay_for_patch_name(Some(patch_name));
                } else {
                    self.append_log_line("patch name JSON が見つかりません".to_string());
                }
            }
            KeyCode::Char('t') => {
                let selected_patch_name = self.selected_history_overlay_item().and_then(|mml| {
                    Self::extract_patch_phrase(&mml).map(|(patch_name, _)| patch_name)
                });
                let current_patch_name = self.current_track_patch_name();
                self.start_patch_select_overlay(
                    selected_patch_name
                        .as_deref()
                        .or(current_patch_name.as_deref()),
                );
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.history_overlay_focus = DawHistoryPane::History;
                self.sync_history_overlay_cursors();
                self.preview_selected_history_overlay_item();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.history_overlay_focus = DawHistoryPane::Favorites;
                self.sync_history_overlay_cursors();
                self.preview_selected_history_overlay_item();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.history_overlay_focus {
                    DawHistoryPane::History
                        if self.history_overlay_history_cursor + 1 < history_len =>
                    {
                        self.history_overlay_history_cursor += 1;
                    }
                    DawHistoryPane::Favorites
                        if self.history_overlay_favorites_cursor + 1 < favorites_len =>
                    {
                        self.history_overlay_favorites_cursor += 1;
                    }
                    _ => {}
                }
                self.sync_history_overlay_cursors();
                self.preview_selected_history_overlay_item();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.history_overlay_focus {
                    DawHistoryPane::History if self.history_overlay_history_cursor > 0 => {
                        self.history_overlay_history_cursor -= 1;
                    }
                    DawHistoryPane::Favorites if self.history_overlay_favorites_cursor > 0 => {
                        self.history_overlay_favorites_cursor -= 1;
                    }
                    _ => {}
                }
                self.sync_history_overlay_cursors();
                self.preview_selected_history_overlay_item();
            }
            KeyCode::Char('/') => {
                self.history_overlay_filter_active = true;
                self.sync_history_overlay_cursors();
            }
            KeyCode::Char('?') => self.enter_help(),
            KeyCode::Enter => {
                if let Some(selected) = self.selected_history_overlay_item() {
                    self.apply_history_overlay_selection(selected);
                }
            }
            KeyCode::Char(' ') => {
                self.preview_selected_history_overlay_item();
            }
            _ => {}
        }
    }
}
