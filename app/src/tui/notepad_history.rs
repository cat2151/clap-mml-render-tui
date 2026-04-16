use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{filter_items, Mode, PatchPhrasePane, TuiApp};

impl<'a> TuiApp<'a> {
    fn move_notepad_selection_by(
        &mut self,
        delta: isize,
        history_len: usize,
        favorites_len: usize,
    ) -> bool {
        match self.notepad_focus {
            PatchPhrasePane::History => {
                if history_len == 0 {
                    return false;
                }
                let max_cursor = history_len.saturating_sub(1) as isize;
                let next_cursor =
                    (self.notepad_history_cursor as isize + delta).clamp(0, max_cursor) as usize;
                if next_cursor == self.notepad_history_cursor {
                    return false;
                }
                self.notepad_history_cursor = next_cursor;
            }
            PatchPhrasePane::Favorites => {
                if favorites_len == 0 {
                    return false;
                }
                let max_cursor = favorites_len.saturating_sub(1) as isize;
                let next_cursor =
                    (self.notepad_favorites_cursor as isize + delta).clamp(0, max_cursor) as usize;
                if next_cursor == self.notepad_favorites_cursor {
                    return false;
                }
                self.notepad_favorites_cursor = next_cursor;
            }
        }
        true
    }

    pub(super) fn notepad_history_items(&self) -> Vec<String> {
        filter_items(
            &self.patch_phrase_store.notepad.history,
            &self.notepad_query,
        )
    }

    pub(super) fn notepad_favorite_items(&self) -> Vec<String> {
        filter_items(
            &self.patch_phrase_store.notepad.favorites,
            &self.notepad_query,
        )
    }

    fn sync_notepad_history_states(&mut self) {
        let history_len = self.notepad_history_items().len();
        if history_len == 0 {
            self.notepad_history_state.select(None);
            self.notepad_history_cursor = 0;
        } else {
            self.notepad_history_cursor = self.notepad_history_cursor.min(history_len - 1);
            self.notepad_history_state
                .select(Some(self.notepad_history_cursor));
            Self::sync_overlay_list_offset(
                &mut self.notepad_history_state,
                self.notepad_history_cursor,
                history_len,
                self.notepad_history_page_size,
            );
        }

        let favorites_len = self.notepad_favorite_items().len();
        if favorites_len == 0 {
            self.notepad_favorites_state.select(None);
            self.notepad_favorites_cursor = 0;
        } else {
            self.notepad_favorites_cursor = self.notepad_favorites_cursor.min(favorites_len - 1);
            self.notepad_favorites_state
                .select(Some(self.notepad_favorites_cursor));
            Self::sync_overlay_list_offset(
                &mut self.notepad_favorites_state,
                self.notepad_favorites_cursor,
                favorites_len,
                self.notepad_history_page_size,
            );
        }
    }

    pub(super) fn record_notepad_history(&mut self, mml: &str) {
        let item = mml.trim();
        if item.is_empty() {
            return;
        }

        Self::push_front_dedup(
            &mut self.patch_phrase_store.notepad.history,
            item.to_string(),
        );
        self.patch_phrase_store_dirty = true;
    }

    pub(super) fn add_notepad_favorite(&mut self, mml: String) {
        if mml.trim().is_empty() {
            return;
        }

        Self::push_front_dedup(&mut self.patch_phrase_store.notepad.favorites, mml);
        self.patch_phrase_store_dirty = true;
    }

    fn selected_notepad_item(&self) -> Option<String> {
        match self.notepad_focus {
            PatchPhrasePane::History => self
                .notepad_history_items()
                .get(self.notepad_history_cursor)
                .cloned(),
            PatchPhrasePane::Favorites => self
                .notepad_favorite_items()
                .get(self.notepad_favorites_cursor)
                .cloned(),
        }
    }

    fn notepad_item_for_selection(&self, focus: PatchPhrasePane, cursor: usize) -> Option<String> {
        match focus {
            PatchPhrasePane::History => self.notepad_history_items().get(cursor).cloned(),
            PatchPhrasePane::Favorites => self.notepad_favorite_items().get(cursor).cloned(),
        }
    }

    fn prefetch_notepad_history_navigation_audio_cache(&self) {
        let (item_count, cursor) = match self.notepad_focus {
            PatchPhrasePane::History => (
                self.notepad_history_items().len(),
                self.notepad_history_cursor,
            ),
            PatchPhrasePane::Favorites => (
                self.notepad_favorite_items().len(),
                self.notepad_favorites_cursor,
            ),
        };
        let focus = self.notepad_focus;
        self.prefetch_navigation_audio_cache(
            cursor,
            item_count,
            self.notepad_history_page_size,
            |index| self.notepad_item_for_selection(focus, index),
        );
    }

    fn preview_selected_notepad_item(&mut self) {
        if let Some(mml) = self.selected_notepad_item() {
            self.play_mml(mml);
            self.prefetch_notepad_history_navigation_audio_cache();
        }
    }

    fn delete_notepad_favorite(&mut self) {
        let Some(selected) = self.selected_notepad_item() else {
            self.notepad_pending_delete = false;
            self.sync_notepad_history_states();
            return;
        };
        let favorites = &mut self.patch_phrase_store.notepad.favorites;
        let Some(index) = favorites.iter().position(|item| item == &selected) else {
            self.notepad_pending_delete = false;
            self.sync_notepad_history_states();
            return;
        };

        let mml = favorites.remove(index);
        Self::push_front_dedup(&mut self.patch_phrase_store.notepad.history, mml);
        self.patch_phrase_store_dirty = true;
        self.notepad_pending_delete = false;
        self.sync_notepad_history_states();
    }

    pub(super) fn start_notepad_history(&mut self) {
        self.notepad_focus = PatchPhrasePane::History;
        self.notepad_history_cursor = 0;
        self.notepad_favorites_cursor = 0;
        self.notepad_query.clear();
        self.notepad_query_textarea = crate::text_input::new_single_line_textarea("");
        self.notepad_filter_active = false;
        self.notepad_pending_delete = false;
        self.sync_notepad_history_states();
        self.mode = Mode::NotepadHistory;
    }

    #[cfg(test)]
    pub(super) fn handle_notepad_history(&mut self, key: KeyCode) {
        self.handle_notepad_history_key_event(KeyEvent::new(key, KeyModifiers::NONE));
    }

    pub(crate) fn handle_notepad_history_key_event(&mut self, key_event: KeyEvent) {
        if self.notepad_filter_active {
            crate::text_input::sync_single_line_textarea(
                &mut self.notepad_query_textarea,
                &self.notepad_query,
            );
            match key_event.code {
                KeyCode::Esc => {
                    self.notepad_pending_delete = false;
                    self.notepad_filter_active = false;
                    self.flush_patch_phrase_store_if_dirty();
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    self.notepad_filter_active = false;
                    self.sync_notepad_history_states();
                }
                KeyCode::Backspace if self.notepad_query.is_empty() => {
                    self.notepad_filter_active = false;
                }
                KeyCode::Char('?') => self.enter_help(),
                _ => {
                    let previous_query = self.notepad_query.clone();
                    if crate::text_input::apply_key_event_to_textarea(
                        &mut self.notepad_query_textarea,
                        key_event,
                    ) {
                        let next_query =
                            crate::text_input::textarea_value(&self.notepad_query_textarea);
                        if next_query == previous_query {
                            return;
                        }
                        self.notepad_query = next_query;
                        self.sync_notepad_history_states();
                        self.preview_selected_notepad_item();
                        if !previous_query.is_empty() && self.notepad_query.is_empty() {
                            self.notepad_filter_active = false;
                        }
                    }
                }
            }
            return;
        }

        if key_event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        let key = key_event.code;

        let was_pending_delete = self.notepad_pending_delete;
        if !(matches!(key, KeyCode::Char('d')) && self.notepad_focus == PatchPhrasePane::Favorites)
        {
            self.notepad_pending_delete = false;
        }

        let history_len = self.notepad_history_items().len();
        let favorites_len = self.notepad_favorite_items().len();

        match key {
            KeyCode::Esc => {
                self.notepad_pending_delete = false;
                self.flush_patch_phrase_store_if_dirty();
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') => {
                // overlay 切替キーを統一するため、notepad history 中でも n で
                // History ペイン先頭・検索解除の初期状態へ戻せるようにする。
                self.start_notepad_history();
            }
            KeyCode::Char('p') => {
                let selected_patch_name = self.selected_notepad_item().and_then(|mml| {
                    Self::extract_patch_phrase(&mml).map(|(patch_name, _)| patch_name)
                });
                self.start_patch_phrase_for_patch_name(
                    selected_patch_name.or_else(|| self.current_line_patch_name()),
                );
            }
            KeyCode::Char('t') => {
                let selected_patch_name = self.selected_notepad_item().and_then(|mml| {
                    Self::extract_patch_phrase(&mml).map(|(patch_name, _)| patch_name)
                });
                let current_patch_name = self.current_line_patch_name();
                self.open_patch_select_overlay(
                    selected_patch_name
                        .as_deref()
                        .or(current_patch_name.as_deref()),
                );
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.notepad_focus = PatchPhrasePane::History;
                self.sync_notepad_history_states();
                self.preview_selected_notepad_item();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.notepad_focus = PatchPhrasePane::Favorites;
                self.sync_notepad_history_states();
                self.preview_selected_notepad_item();
            }
            KeyCode::Char('j') | KeyCode::Down
                if self.move_notepad_selection_by(1, history_len, favorites_len) =>
            {
                self.sync_notepad_history_states();
                self.preview_selected_notepad_item();
            }
            KeyCode::Char('k') | KeyCode::Up
                if self.move_notepad_selection_by(-1, history_len, favorites_len) =>
            {
                self.sync_notepad_history_states();
                self.preview_selected_notepad_item();
            }
            KeyCode::PageDown
                if self.move_notepad_selection_by(
                    self.notepad_history_page_size as isize,
                    history_len,
                    favorites_len,
                ) =>
            {
                self.sync_notepad_history_states();
                self.preview_selected_notepad_item();
            }
            KeyCode::PageUp
                if self.move_notepad_selection_by(
                    -(self.notepad_history_page_size as isize),
                    history_len,
                    favorites_len,
                ) =>
            {
                self.sync_notepad_history_states();
                self.preview_selected_notepad_item();
            }
            KeyCode::Char('/') => {
                self.notepad_filter_active = true;
                self.notepad_pending_delete = false;
                self.notepad_query_textarea =
                    crate::text_input::new_single_line_textarea(&self.notepad_query);
                self.sync_notepad_history_states();
            }
            KeyCode::Enter => {
                if let Some(mml) = self.selected_notepad_item() {
                    self.lines[self.cursor] = mml.clone();
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                    self.flush_patch_phrase_store_if_dirty();
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(' ') => {
                self.preview_selected_notepad_item();
            }
            KeyCode::Char('f') if self.notepad_focus == PatchPhrasePane::History => {
                if let Some(mml) = self.selected_notepad_item() {
                    self.add_notepad_favorite(mml);
                    self.sync_notepad_history_states();
                }
            }
            KeyCode::Char('d') if self.notepad_focus == PatchPhrasePane::Favorites => {
                if was_pending_delete {
                    self.delete_notepad_favorite();
                } else if self.selected_notepad_item().is_some() {
                    self.notepad_pending_delete = true;
                }
            }
            KeyCode::Char('?') => self.enter_help(),
            _ => {}
        }
    }
}
