//! Patch phrase モードの状態遷移と履歴管理

use crossterm::event::KeyCode;
use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;

use super::{filter_items, Mode, PatchPhrasePane, TuiApp, PATCH_JSON_KEY};

const PATCH_PHRASE_LIST_MAX_LEN: usize = 100;

impl<'a> TuiApp<'a> {
    fn filtered_patch_phrase_items(&self, items: Option<&[String]>) -> Vec<String> {
        match items.filter(|items| !items.is_empty()) {
            Some(items) => filter_items(items, &self.patch_phrase_query),
            None => {
                let fallback = [String::from("c")];
                filter_items(&fallback, &self.patch_phrase_query)
            }
        }
    }

    fn move_patch_phrase_selection_by(
        &mut self,
        delta: isize,
        history_len: usize,
        favorites_len: usize,
    ) -> bool {
        match self.patch_phrase_focus {
            PatchPhrasePane::History => {
                if history_len == 0 {
                    return false;
                }
                let max_cursor = history_len.saturating_sub(1) as isize;
                let next_cursor = (self.patch_phrase_history_cursor as isize + delta)
                    .clamp(0, max_cursor) as usize;
                if next_cursor == self.patch_phrase_history_cursor {
                    return false;
                }
                self.patch_phrase_history_cursor = next_cursor;
            }
            PatchPhrasePane::Favorites => {
                if favorites_len == 0 {
                    return false;
                }
                let max_cursor = favorites_len.saturating_sub(1) as isize;
                let next_cursor = (self.patch_phrase_favorites_cursor as isize + delta)
                    .clamp(0, max_cursor) as usize;
                if next_cursor == self.patch_phrase_favorites_cursor {
                    return false;
                }
                self.patch_phrase_favorites_cursor = next_cursor;
            }
        }
        true
    }

    pub(super) fn push_front_dedup(items: &mut Vec<String>, item: String) {
        if let Some(index) = items.iter().position(|existing| existing == &item) {
            if index == 0 {
                return;
            }
            items.remove(index);
        }
        items.insert(0, item);
        if items.len() > PATCH_PHRASE_LIST_MAX_LEN {
            items.truncate(PATCH_PHRASE_LIST_MAX_LEN);
        }
    }

    pub(super) fn extract_patch_phrase(mml: &str) -> Option<(String, String)> {
        let preprocessed = mml_preprocessor::extract_embedded_json(mml);
        let patch_name = preprocessed
            .embedded_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
            .and_then(|value| {
                value
                    .get(PATCH_JSON_KEY)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })?;
        let phrase = preprocessed.remaining_mml.trim().to_string();
        Some((patch_name, phrase))
    }

    pub(super) fn patch_phrase_history_items(&self) -> Vec<String> {
        self.filtered_patch_phrase_items(
            self.patch_phrase_name
                .as_deref()
                .and_then(|patch| self.patch_phrase_store.patches.get(patch))
                .map(|state| state.history.as_slice()),
        )
    }

    pub(super) fn patch_phrase_favorite_items(&self) -> Vec<String> {
        self.filtered_patch_phrase_items(
            self.patch_phrase_name
                .as_deref()
                .and_then(|patch| self.patch_phrase_store.patches.get(patch))
                .map(|state| state.favorites.as_slice()),
        )
    }

    fn sync_patch_phrase_states(&mut self) {
        let history_len = self.patch_phrase_history_items().len();
        if history_len == 0 {
            self.patch_phrase_history_cursor = 0;
            self.patch_phrase_history_state.select(None);
        } else {
            self.patch_phrase_history_cursor =
                self.patch_phrase_history_cursor.min(history_len - 1);
            self.patch_phrase_history_state
                .select(Some(self.patch_phrase_history_cursor));
        }

        let favorites_len = self.patch_phrase_favorite_items().len();
        if favorites_len == 0 {
            self.patch_phrase_favorites_cursor = 0;
            self.patch_phrase_favorites_state.select(None);
        } else {
            self.patch_phrase_favorites_cursor =
                self.patch_phrase_favorites_cursor.min(favorites_len - 1);
            self.patch_phrase_favorites_state
                .select(Some(self.patch_phrase_favorites_cursor));
        }
    }

    pub(super) fn flush_patch_phrase_store_if_dirty(&mut self) {
        if !self.patch_phrase_store_dirty {
            return;
        }
        let _ = crate::history::save_patch_phrase_store(&self.patch_phrase_store);
        self.patch_phrase_store_dirty = false;
    }

    pub(super) fn record_patch_phrase_history(&mut self, mml: &str) {
        let Some((patch_name, phrase)) = Self::extract_patch_phrase(mml) else {
            return;
        };
        if phrase.is_empty() {
            return;
        }
        let patch_name = self.normalize_patch_phrase_store_key(patch_name);

        let state = self
            .patch_phrase_store
            .patches
            .entry(patch_name)
            .or_default();
        Self::push_front_dedup(&mut state.history, phrase);
        self.patch_phrase_store_dirty = true;
    }

    pub(super) fn add_patch_phrase_favorite(&mut self, patch_name: String, phrase: String) {
        let patch_name = self.normalize_patch_phrase_store_key(patch_name);
        let state = self
            .patch_phrase_store
            .patches
            .entry(patch_name.clone())
            .or_default();
        Self::push_front_dedup(&mut state.favorites, phrase);
        crate::history::touch_patch_favorite(&mut self.patch_phrase_store, &patch_name);
        self.patch_phrase_store_dirty = true;
    }

    fn patch_phrase_preview_mml(&self) -> Option<String> {
        let patch_name = self.patch_phrase_name.as_deref()?;
        let phrase = match self.patch_phrase_focus {
            PatchPhrasePane::History => self
                .patch_phrase_history_items()
                .get(self.patch_phrase_history_cursor)
                .cloned(),
            PatchPhrasePane::Favorites => self
                .patch_phrase_favorite_items()
                .get(self.patch_phrase_favorites_cursor)
                .cloned(),
        }?;
        let json = serde_json::json!({ PATCH_JSON_KEY: patch_name }).to_string();
        Some(format!("{json} {phrase}"))
    }

    pub(super) fn start_patch_phrase(&mut self, patch_name: String) {
        let ready_pairs = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                crate::tui::PatchLoadState::Ready(pairs) => Some(pairs.clone()),
                crate::tui::PatchLoadState::Loading | crate::tui::PatchLoadState::Err(_) => None,
            }
        };
        if let Some(pairs) = ready_pairs.as_deref() {
            self.normalize_patch_phrase_store_for_available_patches(pairs);
        }
        let patch_name = self.normalize_patch_phrase_store_key(patch_name);
        self.patch_phrase_name = Some(patch_name);
        self.patch_phrase_focus = PatchPhrasePane::History;
        self.patch_phrase_history_cursor = 0;
        self.patch_phrase_favorites_cursor = 0;
        self.patch_phrase_query.clear();
        self.patch_phrase_filter_active = false;
        self.sync_patch_phrase_states();
        self.mode = Mode::PatchPhrase;
    }

    pub(super) fn handle_patch_phrase(&mut self, key: KeyCode) {
        if self.patch_phrase_filter_active {
            match key {
                KeyCode::Esc => {
                    self.patch_phrase_filter_active = false;
                    self.flush_patch_phrase_store_if_dirty();
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    self.patch_phrase_filter_active = false;
                    self.sync_patch_phrase_states();
                }
                KeyCode::Backspace => {
                    self.patch_phrase_query.pop();
                    self.sync_patch_phrase_states();
                    if let Some(mml) = self.patch_phrase_preview_mml() {
                        self.record_notepad_history(&mml);
                        self.play_mml(mml);
                    }
                    if self.patch_phrase_query.is_empty() {
                        self.patch_phrase_filter_active = false;
                    }
                }
                KeyCode::Char('?') => self.enter_help(),
                KeyCode::Char(c) => {
                    self.patch_phrase_query.push(c);
                    self.sync_patch_phrase_states();
                    if let Some(mml) = self.patch_phrase_preview_mml() {
                        self.record_notepad_history(&mml);
                        self.play_mml(mml);
                    }
                }
                _ => {}
            }
            return;
        }

        let history_len = self.patch_phrase_history_items().len();
        let favorites_len = self.patch_phrase_favorite_items().len();

        match key {
            KeyCode::Esc => {
                self.flush_patch_phrase_store_if_dirty();
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') => {
                self.start_notepad_history();
            }
            KeyCode::Char('p') => {
                // overlay 切替キーを統一するため、patch history 中でも p で
                // 現在 patch の History 先頭・検索解除の初期状態へ戻せるようにする。
                self.start_patch_phrase_for_patch_name(self.patch_phrase_name.clone());
            }
            KeyCode::Char('t') => {
                let patch_name = self.patch_phrase_name.clone();
                self.open_patch_select_overlay(patch_name.as_deref());
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.patch_phrase_focus = PatchPhrasePane::History;
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.patch_phrase_focus = PatchPhrasePane::Favorites;
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.move_patch_phrase_selection_by(1, history_len, favorites_len) {
                    self.sync_patch_phrase_states();
                    if let Some(mml) = self.patch_phrase_preview_mml() {
                        self.record_notepad_history(&mml);
                        self.play_mml(mml);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.move_patch_phrase_selection_by(-1, history_len, favorites_len) {
                    self.sync_patch_phrase_states();
                    if let Some(mml) = self.patch_phrase_preview_mml() {
                        self.record_notepad_history(&mml);
                        self.play_mml(mml);
                    }
                }
            }
            KeyCode::PageDown => {
                if self.move_patch_phrase_selection_by(
                    self.patch_phrase_page_size as isize,
                    history_len,
                    favorites_len,
                ) {
                    self.sync_patch_phrase_states();
                    if let Some(mml) = self.patch_phrase_preview_mml() {
                        self.record_notepad_history(&mml);
                        self.play_mml(mml);
                    }
                }
            }
            KeyCode::PageUp => {
                if self.move_patch_phrase_selection_by(
                    -(self.patch_phrase_page_size as isize),
                    history_len,
                    favorites_len,
                ) {
                    self.sync_patch_phrase_states();
                    if let Some(mml) = self.patch_phrase_preview_mml() {
                        self.record_notepad_history(&mml);
                        self.play_mml(mml);
                    }
                }
            }
            KeyCode::Char('/') => {
                self.patch_phrase_filter_active = true;
                self.sync_patch_phrase_states();
            }
            KeyCode::Enter => {
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.lines.insert(self.cursor, mml.clone());
                    self.list_state.select(Some(self.cursor));
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                    self.flush_patch_phrase_store_if_dirty();
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(' ') => {
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                }
            }
            KeyCode::Char('i') if self.patch_phrase_focus == PatchPhrasePane::History => {
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.lines[self.cursor] = mml;
                    self.start_insert();
                }
            }
            KeyCode::Char('f') => {
                let Some(patch_name) = self.patch_phrase_name.clone() else {
                    return;
                };
                let phrase = match self.patch_phrase_focus {
                    PatchPhrasePane::History => self
                        .patch_phrase_history_items()
                        .get(self.patch_phrase_history_cursor)
                        .cloned(),
                    PatchPhrasePane::Favorites => self
                        .patch_phrase_favorite_items()
                        .get(self.patch_phrase_favorites_cursor)
                        .cloned(),
                };
                let Some(phrase) = phrase else {
                    return;
                };
                self.add_patch_phrase_favorite(patch_name, phrase);
                self.patch_phrase_focus = PatchPhrasePane::Favorites;
                self.patch_phrase_favorites_cursor = 0;
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                }
            }
            KeyCode::Char('?') => self.enter_help(),
            _ => {}
        }
    }
}
