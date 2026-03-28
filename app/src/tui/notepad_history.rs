use crossterm::event::KeyCode;

use super::{Mode, PatchPhrasePane, TuiApp};

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
        self.patch_phrase_store.notepad.history.clone()
    }

    pub(super) fn notepad_favorite_items(&self) -> Vec<String> {
        self.patch_phrase_store.notepad.favorites.clone()
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
        }

        let favorites_len = self.notepad_favorite_items().len();
        if favorites_len == 0 {
            self.notepad_favorites_state.select(None);
            self.notepad_favorites_cursor = 0;
        } else {
            self.notepad_favorites_cursor = self.notepad_favorites_cursor.min(favorites_len - 1);
            self.notepad_favorites_state
                .select(Some(self.notepad_favorites_cursor));
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

    fn preview_selected_notepad_item(&mut self) {
        if let Some(mml) = self.selected_notepad_item() {
            self.play_mml(mml);
        }
    }

    fn delete_notepad_favorite(&mut self) {
        let favorites = &mut self.patch_phrase_store.notepad.favorites;
        if self.notepad_favorites_cursor >= favorites.len() {
            self.notepad_pending_delete = false;
            self.sync_notepad_history_states();
            return;
        }

        let mml = favorites.remove(self.notepad_favorites_cursor);
        Self::push_front_dedup(&mut self.patch_phrase_store.notepad.history, mml);
        self.patch_phrase_store_dirty = true;
        self.notepad_pending_delete = false;
        self.sync_notepad_history_states();
    }

    pub(super) fn start_notepad_history(&mut self) {
        self.notepad_focus = PatchPhrasePane::History;
        self.notepad_history_cursor = 0;
        self.notepad_favorites_cursor = 0;
        self.notepad_pending_delete = false;
        self.sync_notepad_history_states();
        self.mode = Mode::NotepadHistory;
    }

    pub(super) fn handle_notepad_history(&mut self, key: KeyCode) {
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
            KeyCode::Char('j') | KeyCode::Down => {
                if self.move_notepad_selection_by(1, history_len, favorites_len) {
                    self.sync_notepad_history_states();
                    self.preview_selected_notepad_item();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.move_notepad_selection_by(-1, history_len, favorites_len) {
                    self.sync_notepad_history_states();
                    self.preview_selected_notepad_item();
                }
            }
            KeyCode::PageDown => {
                if self.move_notepad_selection_by(
                    self.notepad_history_page_size as isize,
                    history_len,
                    favorites_len,
                ) {
                    self.sync_notepad_history_states();
                    self.preview_selected_notepad_item();
                }
            }
            KeyCode::PageUp => {
                if self.move_notepad_selection_by(
                    -(self.notepad_history_page_size as isize),
                    history_len,
                    favorites_len,
                ) {
                    self.sync_notepad_history_states();
                    self.preview_selected_notepad_item();
                }
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
            _ => {}
        }
    }
}
