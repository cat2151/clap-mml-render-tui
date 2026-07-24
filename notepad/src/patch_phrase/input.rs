//! Patch phrase モードの状態遷移と履歴管理

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;

use super::super::{filter_items, Mode, PatchPhrasePane, PATCH_JSON_KEY};
use crate::NotepadScreen;

const PATCH_PHRASE_LIST_MAX_LEN: usize = 100;

impl<'a> NotepadScreen<'a> {
    fn filtered_patch_phrase_items(&self, items: Option<&[String]>) -> Vec<String> {
        match items.filter(|items| !items.is_empty()) {
            Some(items) => filter_items(items, &self.patch_phrase.query),
            None => {
                let fallback = [String::from("c")];
                filter_items(&fallback, &self.patch_phrase.query)
            }
        }
    }

    fn move_patch_phrase_selection_by(
        &mut self,
        delta: isize,
        history_len: usize,
        favorites_len: usize,
    ) -> bool {
        match self.patch_phrase.focus {
            PatchPhrasePane::History => {
                if history_len == 0 {
                    return false;
                }
                let max_cursor = history_len.saturating_sub(1) as isize;
                let next_cursor = (self.patch_phrase.history_cursor as isize + delta)
                    .clamp(0, max_cursor) as usize;
                if next_cursor == self.patch_phrase.history_cursor {
                    return false;
                }
                self.patch_phrase.history_cursor = next_cursor;
            }
            PatchPhrasePane::Favorites => {
                if favorites_len == 0 {
                    return false;
                }
                let max_cursor = favorites_len.saturating_sub(1) as isize;
                let next_cursor = (self.patch_phrase.favorites_cursor as isize + delta)
                    .clamp(0, max_cursor) as usize;
                if next_cursor == self.patch_phrase.favorites_cursor {
                    return false;
                }
                self.patch_phrase.favorites_cursor = next_cursor;
            }
        }
        true
    }

    pub(crate) fn push_front_dedup(items: &mut Vec<String>, item: String) {
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

    pub(crate) fn extract_patch_phrase(mml: &str) -> Option<(String, String)> {
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

    pub(crate) fn patch_phrase_history_items(&self) -> Vec<String> {
        self.filtered_patch_phrase_items(
            self.patch_phrase
                .patch_name
                .as_deref()
                .and_then(|patch| self.patch_phrase_store.patches.get(patch))
                .map(|state| state.history.as_slice()),
        )
    }

    pub(crate) fn patch_phrase_favorite_items(&self) -> Vec<String> {
        self.filtered_patch_phrase_items(
            self.patch_phrase
                .patch_name
                .as_deref()
                .and_then(|patch| self.patch_phrase_store.patches.get(patch))
                .map(|state| state.favorites.as_slice()),
        )
    }

    fn sync_patch_phrase_states(&mut self) {
        let history_len = self.patch_phrase_history_items().len();
        if history_len == 0 {
            self.patch_phrase.history_cursor = 0;
            self.patch_phrase.history_state.select(None);
        } else {
            self.patch_phrase.history_cursor =
                self.patch_phrase.history_cursor.min(history_len - 1);
            self.patch_phrase
                .history_state
                .select(Some(self.patch_phrase.history_cursor));
            Self::sync_overlay_list_offset(
                &mut self.patch_phrase.history_state,
                self.patch_phrase.history_cursor,
                history_len,
                self.patch_phrase.page_size,
            );
        }

        let favorites_len = self.patch_phrase_favorite_items().len();
        if favorites_len == 0 {
            self.patch_phrase.favorites_cursor = 0;
            self.patch_phrase.favorites_state.select(None);
        } else {
            self.patch_phrase.favorites_cursor =
                self.patch_phrase.favorites_cursor.min(favorites_len - 1);
            self.patch_phrase
                .favorites_state
                .select(Some(self.patch_phrase.favorites_cursor));
            Self::sync_overlay_list_offset(
                &mut self.patch_phrase.favorites_state,
                self.patch_phrase.favorites_cursor,
                favorites_len,
                self.patch_phrase.page_size,
            );
        }
    }

    pub fn flush_patch_phrase_store_if_dirty(&mut self) {
        if !self.patch_phrase_store_dirty {
            return;
        }
        let _ = cmrt_history::save_patch_phrase_store(&self.patch_phrase_store);
        self.patch_phrase_store_dirty = false;
    }

    pub(crate) fn record_patch_phrase_history(&mut self, mml: &str) {
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

    pub(crate) fn add_patch_phrase_favorite(&mut self, patch_name: String, phrase: String) {
        let patch_name = self.normalize_patch_phrase_store_key(patch_name);
        let state = self
            .patch_phrase_store
            .patches
            .entry(patch_name.clone())
            .or_default();
        Self::push_front_dedup(&mut state.favorites, phrase);
        cmrt_history::touch_patch_favorite(&mut self.patch_phrase_store, &patch_name);
        self.patch_phrase_store_dirty = true;
    }

    pub(crate) fn patch_phrase_preview_mml_for_selection(
        &self,
        focus: PatchPhrasePane,
        cursor: usize,
    ) -> Option<String> {
        let patch_name = self.patch_phrase.patch_name.as_deref()?;
        let phrase = match focus {
            PatchPhrasePane::History => self.patch_phrase_history_items().get(cursor).cloned(),
            PatchPhrasePane::Favorites => self.patch_phrase_favorite_items().get(cursor).cloned(),
        }?;
        let json = serde_json::json!({ PATCH_JSON_KEY: patch_name }).to_string();
        Some(format!("{json} {phrase}"))
    }

    fn patch_phrase_preview_mml(&self) -> Option<String> {
        let cursor = match self.patch_phrase.focus {
            PatchPhrasePane::History => self.patch_phrase.history_cursor,
            PatchPhrasePane::Favorites => self.patch_phrase.favorites_cursor,
        };
        self.patch_phrase_preview_mml_for_selection(self.patch_phrase.focus, cursor)
    }

    fn prefetch_patch_phrase_navigation_audio_cache(&self, preferred_delta: Option<isize>) {
        let (item_count, cursor) = match self.patch_phrase.focus {
            PatchPhrasePane::History => (
                self.patch_phrase_history_items().len(),
                self.patch_phrase.history_cursor,
            ),
            PatchPhrasePane::Favorites => (
                self.patch_phrase_favorite_items().len(),
                self.patch_phrase.favorites_cursor,
            ),
        };
        let focus = self.patch_phrase.focus;
        self.prefetch_navigation_audio_cache(
            cursor,
            item_count,
            self.patch_phrase.page_size,
            preferred_delta,
            |index| self.patch_phrase_preview_mml_for_selection(focus, index),
        );
    }

    fn preview_selected_patch_phrase_item(&mut self) {
        self.preview_selected_patch_phrase_item_with_navigation_hint(None);
    }

    fn preview_selected_patch_phrase_item_with_navigation_hint(
        &mut self,
        preferred_delta: Option<isize>,
    ) {
        if let Some(mml) = self.patch_phrase_preview_mml() {
            self.record_notepad_history(&mml);
            self.play_mml(mml);
            self.prefetch_patch_phrase_navigation_audio_cache(preferred_delta);
        }
    }

    pub(crate) fn start_patch_phrase(&mut self, patch_name: String) {
        let ready_pairs = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                crate::PatchLoadState::Ready(pairs) => Some(pairs.clone()),
                crate::PatchLoadState::Loading | crate::PatchLoadState::Err(_) => None,
            }
        };
        if let Some(pairs) = ready_pairs.as_deref() {
            self.normalize_patch_phrase_store_for_available_patches(pairs);
        }
        let patch_name = self.normalize_patch_phrase_store_key(patch_name);
        self.patch_phrase.patch_name = Some(patch_name);
        self.patch_phrase.focus = PatchPhrasePane::History;
        self.patch_phrase.history_cursor = 0;
        self.patch_phrase.favorites_cursor = 0;
        self.patch_phrase.query.clear();
        self.patch_phrase.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea("");
        self.patch_phrase.filter_active = false;
        self.sync_patch_phrase_states();
        self.mode = Mode::PatchPhrase;
    }

    #[cfg(test)]
    pub(crate) fn handle_patch_phrase(&mut self, key: KeyCode) {
        self.handle_patch_phrase_key_event(KeyEvent::new(key, KeyModifiers::NONE));
    }

    pub(crate) fn handle_patch_phrase_key_event(&mut self, key_event: KeyEvent) {
        if self.patch_phrase.filter_active {
            cmrt_tui_core::text_input::sync_single_line_textarea(
                &mut self.patch_phrase.query_textarea,
                &self.patch_phrase.query,
            );
            match key_event.code {
                KeyCode::Esc => {
                    self.patch_phrase.filter_active = false;
                    self.flush_patch_phrase_store_if_dirty();
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    self.patch_phrase.filter_active = false;
                    self.sync_patch_phrase_states();
                }
                KeyCode::Backspace if self.patch_phrase.query.is_empty() => {
                    self.patch_phrase.filter_active = false;
                }
                KeyCode::Char('?') => self.enter_help(),
                _ => {
                    let previous_query = self.patch_phrase.query.clone();
                    if cmrt_tui_core::text_input::apply_key_event_to_textarea(
                        &mut self.patch_phrase.query_textarea,
                        key_event,
                    ) {
                        let next_query = cmrt_tui_core::text_input::textarea_value(
                            &self.patch_phrase.query_textarea,
                        );
                        if next_query == previous_query {
                            return;
                        }
                        self.patch_phrase.query = next_query;
                        self.sync_patch_phrase_states();
                        self.preview_selected_patch_phrase_item();
                        if !previous_query.is_empty() && self.patch_phrase.query.is_empty() {
                            self.patch_phrase.filter_active = false;
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
                self.start_patch_phrase_for_patch_name(self.patch_phrase.patch_name.clone());
            }
            KeyCode::Char('t') => {
                let patch_name = self.patch_phrase.patch_name.clone();
                self.open_patch_select_overlay(patch_name.as_deref());
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.patch_phrase.focus = PatchPhrasePane::History;
                self.sync_patch_phrase_states();
                self.preview_selected_patch_phrase_item();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.patch_phrase.focus = PatchPhrasePane::Favorites;
                self.sync_patch_phrase_states();
                self.preview_selected_patch_phrase_item();
            }
            KeyCode::Char('j') | KeyCode::Down
                if self.move_patch_phrase_selection_by(1, history_len, favorites_len) =>
            {
                self.sync_patch_phrase_states();
                self.preview_selected_patch_phrase_item_with_navigation_hint(Some(1));
            }
            KeyCode::Char('k') | KeyCode::Up
                if self.move_patch_phrase_selection_by(-1, history_len, favorites_len) =>
            {
                self.sync_patch_phrase_states();
                self.preview_selected_patch_phrase_item_with_navigation_hint(Some(-1));
            }
            KeyCode::PageDown
                if self.move_patch_phrase_selection_by(
                    self.patch_phrase.page_size as isize,
                    history_len,
                    favorites_len,
                ) =>
            {
                self.sync_patch_phrase_states();
                self.preview_selected_patch_phrase_item_with_navigation_hint(Some(
                    self.patch_phrase.page_size as isize,
                ));
            }
            KeyCode::PageUp
                if self.move_patch_phrase_selection_by(
                    -(self.patch_phrase.page_size as isize),
                    history_len,
                    favorites_len,
                ) =>
            {
                self.sync_patch_phrase_states();
                self.preview_selected_patch_phrase_item_with_navigation_hint(Some(
                    -(self.patch_phrase.page_size as isize),
                ));
            }
            KeyCode::Char('/') => {
                self.patch_phrase.filter_active = true;
                self.patch_phrase.query_textarea =
                    cmrt_tui_core::text_input::new_single_line_textarea(&self.patch_phrase.query);
                self.sync_patch_phrase_states();
            }
            KeyCode::Enter => {
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.editor.lines.insert(self.editor.cursor, mml.clone());
                    self.editor.list_state.select(Some(self.editor.cursor));
                    self.record_notepad_history(&mml);
                    self.play_mml(mml);
                    self.flush_patch_phrase_store_if_dirty();
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(' ') => {
                self.preview_selected_patch_phrase_item();
            }
            KeyCode::Char('i') if self.patch_phrase.focus == PatchPhrasePane::History => {
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.editor.lines[self.editor.cursor] = mml;
                    self.start_insert();
                }
            }
            KeyCode::Char('f') => {
                let Some(patch_name) = self.patch_phrase.patch_name.clone() else {
                    return;
                };
                let phrase = match self.patch_phrase.focus {
                    PatchPhrasePane::History => self
                        .patch_phrase_history_items()
                        .get(self.patch_phrase.history_cursor)
                        .cloned(),
                    PatchPhrasePane::Favorites => self
                        .patch_phrase_favorite_items()
                        .get(self.patch_phrase.favorites_cursor)
                        .cloned(),
                };
                let Some(phrase) = phrase else {
                    return;
                };
                self.add_patch_phrase_favorite(patch_name, phrase);
                self.patch_phrase.focus = PatchPhrasePane::Favorites;
                self.patch_phrase.favorites_cursor = 0;
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
