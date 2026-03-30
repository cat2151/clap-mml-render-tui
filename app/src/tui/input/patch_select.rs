use crossterm::event::{KeyCode, KeyModifiers};
use mmlabc_to_smf::mml_preprocessor;
use ratatui::widgets::ListState;
use std::time::{SystemTime, UNIX_EPOCH};

use tui_textarea::TextArea;

use crate::tui::{filter_patches, PatchLoadState, PatchSelectPane, PATCH_JSON_KEY};

use super::{Mode, PlayState, TuiApp};

const PATCH_SELECT_PREVIEW_FALLBACK_PHRASE: &str = "c";

impl<'a> TuiApp<'a> {
    fn move_patch_cursor_by(&mut self, delta: isize) {
        if self.patch_filtered.is_empty() {
            return;
        }
        let max_cursor = self.patch_filtered.len().saturating_sub(1) as isize;
        let next_cursor = (self.patch_cursor as isize + delta).clamp(0, max_cursor) as usize;
        if next_cursor != self.patch_cursor {
            self.patch_cursor = next_cursor;
            self.patch_list_state.select(Some(self.patch_cursor));
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
            self.preview_selected_patch();
        }
    }

    fn move_patch_select_selection_by(&mut self, delta: isize) {
        match self.patch_select_focus {
            PatchSelectPane::Patches => self.move_patch_cursor_by(delta),
            PatchSelectPane::Favorites => self.move_patch_favorites_cursor_by(delta),
        }
    }

    fn build_patch_json(patch_name: &str) -> String {
        format!(
            "{{\"{PATCH_JSON_KEY}\": {}}}",
            serde_json::to_string(patch_name).unwrap_or_else(|_| format!("\"{}\"", patch_name))
        )
    }

    pub(super) fn replace_current_line_patch(&mut self, patch_name: &str) {
        let json = Self::build_patch_json(patch_name);
        let current = self.lines[self.cursor].clone();
        let replaced_parts = current
            .split(';')
            .map(|part| {
                let part = part.trim_start();
                let preprocessed = mml_preprocessor::extract_embedded_json(part);
                let remaining = preprocessed.remaining_mml.trim();
                if remaining.is_empty() {
                    String::new()
                } else {
                    format!("{json} {remaining}")
                }
            })
            .collect::<Vec<_>>();
        let replaced = replaced_parts.join(";");
        let has_content = replaced_parts.iter().any(|part| !part.trim().is_empty());
        self.lines[self.cursor] = if has_content {
            replaced
        } else {
            format!("{json} c")
        };
    }

    pub(in crate::tui) fn play_mml(&mut self, mml: String) {
        #[cfg(test)]
        if self.entry_ptr == 0 {
            // new_for_test() では PluginEntry を持たないため、
            // テスト中は再生スレッドを起動せず play_state 更新だけを検証する。
            *self.play_state.lock().unwrap() = PlayState::Running(mml);
            return;
        }

        self.kick_play(mml);
    }

    pub(super) fn play_current_line(&mut self) {
        let mml = self.lines[self.cursor].trim().to_string();
        if !mml.is_empty() {
            self.record_notepad_history(&mml);
            self.record_patch_phrase_history(&mml);
            self.play_mml(mml);
        }
    }

    pub(super) fn insert_generated_line_above(&mut self) -> Result<(), String> {
        let patch_name = self.pick_random_patch_name()?;
        let mml = format!(
            "{} {}",
            Self::build_patch_json(&patch_name),
            crate::generate::pick_default_generate_phrase()
        );
        self.lines.insert(self.cursor, mml.clone());
        self.list_state.select(Some(self.cursor));
        self.record_notepad_history(&mml);
        self.record_patch_phrase_history(&mml);
        self.play_mml(mml);
        Ok(())
    }

    pub(super) fn pick_random_patch_name(&self) -> Result<String, String> {
        if self.cfg.patches_dir.is_none() {
            return Err("patches_dir が設定されていません".to_string());
        }
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
            PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
            PatchLoadState::Ready(pairs) if pairs.is_empty() => {
                Err("patches_dir にパッチが見つかりません".to_string())
            }
            PatchLoadState::Ready(pairs) => {
                let ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0);
                let index = (ns % pairs.len() as u128) as usize;
                Ok(pairs[index].0.clone())
            }
        }
    }

    pub(in crate::tui) fn start_insert(&mut self) {
        self.textarea = TextArea::default();
        let current = self.lines[self.cursor].clone();
        for ch in current.chars() {
            self.textarea.insert_char(ch);
        }
        self.mode = Mode::Insert;
    }

    pub(super) fn insert_empty_line_and_start_insert(&mut self, index: usize) {
        self.lines.insert(index, String::new());
        self.cursor = index;
        self.list_state.select(Some(self.cursor));
        self.start_insert();
    }

    pub(super) fn delete_current_line(&mut self) {
        self.yank_buffer = Some(self.lines.remove(self.cursor));
        if self.lines.is_empty() {
            self.lines.push(String::new());
            self.cursor = 0;
        } else if self.cursor >= self.lines.len() {
            self.cursor = self.lines.len().saturating_sub(1);
        }
        self.list_state.select(Some(self.cursor));
    }

    pub(super) fn paste_yanked_line(&mut self, insert_above: bool) -> bool {
        let Some(yanked) = self.yank_buffer.as_ref() else {
            return false;
        };
        let insert_at = if insert_above {
            self.cursor
        } else {
            self.cursor + 1
        };
        self.lines.insert(insert_at, yanked.clone());
        self.cursor = insert_at;
        self.list_state.select(Some(self.cursor));
        true
    }

    pub(super) fn start_patch_phrase_for_current_line(&mut self) {
        self.start_patch_phrase_for_patch_name(self.current_line_patch_name());
    }

    pub(in crate::tui) fn current_line_patch_name(&self) -> Option<String> {
        self.lines
            .get(self.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
    }

    pub(in crate::tui) fn start_patch_phrase_for_patch_name(&mut self, patch_name: Option<String>) {
        match patch_name {
            Some(patch_name) => self.start_patch_phrase(patch_name),
            None => {
                *self.play_state.lock().unwrap() =
                    PlayState::Err("patch name JSON が見つかりません".to_string());
            }
        }
    }

    fn start_patch_select_with_initial_patch_name(&mut self, initial_patch_name: Option<&str>) {
        // ロードが完了したパッチリストをスナップショットする
        {
            let state = self.patch_load_state.lock().unwrap();
            if let PatchLoadState::Ready(pairs) = &*state {
                self.patch_all = pairs.clone();
            }
        }
        self.patch_query = String::new();
        self.patch_filtered = self
            .patch_all
            .iter()
            .map(|(orig, _)| orig.clone())
            .collect();
        self.patch_select_focus = PatchSelectPane::Patches;
        self.patch_select_filter_active = false;
        self.patch_cursor = initial_patch_name
            .map(str::to_string)
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
    }

    pub(in crate::tui) fn start_patch_select(&mut self) {
        self.start_patch_select_with_initial_patch_name(None);
    }

    pub(in crate::tui) fn open_patch_select_overlay(&mut self, initial_patch_name: Option<&str>) {
        if self.cfg.patches_dir.is_none() {
            *self.play_state.lock().unwrap() =
                PlayState::Err("patches_dir が設定されていません".to_string());
            return;
        }

        let action = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
                PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
                PatchLoadState::Ready(p) if p.is_empty() => {
                    Err("patches_dir にパッチが見つかりません".to_string())
                }
                PatchLoadState::Ready(_) => Ok(()),
            }
        };

        match action {
            Err(msg) => *self.play_state.lock().unwrap() = PlayState::Err(msg),
            Ok(()) => match initial_patch_name {
                Some(patch_name) => {
                    self.start_patch_select_with_initial_patch_name(Some(patch_name))
                }
                None => self.start_patch_select(),
            },
        }
    }

    pub(super) fn set_empty_yank_error(&mut self) {
        *self.play_state.lock().unwrap() = PlayState::Err("yank バッファが空です".to_string());
    }

    fn patch_select_current_phrase(&self) -> Option<String> {
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

        favorites
    }

    fn refresh_patch_select_favorites(&mut self) {
        self.patch_favorite_items = self.rebuild_patch_select_favorite_items();
    }

    pub(in crate::tui) fn patch_select_favorite_items(&self) -> &[String] {
        &self.patch_favorite_items
    }

    fn sync_patch_select_states(&mut self) {
        if self.patch_filtered.is_empty() {
            self.patch_cursor = 0;
            self.patch_list_state.select(None);
        } else {
            self.patch_cursor = self.patch_cursor.min(self.patch_filtered.len() - 1);
            self.patch_list_state.select(Some(self.patch_cursor));
        }

        let favorites_len = self.patch_favorite_items.len();
        if favorites_len == 0 {
            self.patch_favorites_cursor = 0;
            self.patch_favorites_state.select(None);
        } else {
            self.patch_favorites_cursor = self.patch_favorites_cursor.min(favorites_len - 1);
            self.patch_favorites_state
                .select(Some(self.patch_favorites_cursor));
        }
    }

    fn patch_select_selected_patch_name(&self) -> Option<String> {
        match self.patch_select_focus {
            PatchSelectPane::Patches => self.patch_filtered.get(self.patch_cursor).cloned(),
            PatchSelectPane::Favorites => self
                .patch_favorite_items
                .get(self.patch_favorites_cursor)
                .cloned(),
        }
    }

    fn patch_select_preview_mml(&self) -> Option<String> {
        let patch_name = self.patch_select_selected_patch_name()?;
        let phrase = self.patch_select_current_phrase()?;
        let json = Self::build_patch_json(&patch_name);
        Some(format!("{json} {phrase}"))
    }

    fn preview_selected_patch(&mut self) {
        if let Some(mml) = self.patch_select_preview_mml() {
            self.record_notepad_history(&mml);
            self.play_mml(mml);
        }
    }

    pub(super) fn update_patch_filter(&mut self) {
        self.patch_filtered = filter_patches(&self.patch_all, &self.patch_query);
        self.patch_cursor = 0;
        self.sync_patch_select_states();
        self.preview_selected_patch();
    }

    pub(in crate::tui) fn handle_patch_select(&mut self, key_event: crossterm::event::KeyEvent) {
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = key_event.code {
                match c.to_ascii_lowercase() {
                    'f' => {
                        let Some(patch_name) = self.patch_select_selected_patch_name() else {
                            return;
                        };
                        let Some(phrase) = self.patch_select_current_phrase() else {
                            return;
                        };
                        self.add_patch_phrase_favorite(patch_name, phrase);
                        self.refresh_patch_select_favorites();
                        self.sync_patch_select_states();
                        self.preview_selected_patch();
                    }
                    'j' | 'n' => {
                        self.move_patch_select_selection_by(1);
                    }
                    'k' | 'p' => {
                        self.move_patch_select_selection_by(-1);
                    }
                    _ => {}
                }
            }
            return;
        }

        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') if !self.patch_select_filter_active => {
                self.start_notepad_history();
            }
            KeyCode::Char('p') if !self.patch_select_filter_active => {
                self.start_patch_phrase_for_patch_name(self.patch_select_selected_patch_name());
            }
            KeyCode::Char('t') if !self.patch_select_filter_active => {
                // overlay 切替キーを統一するため、音色選択中でも t で現在選択に揃えて開き直せるようにする。
                let selected_patch_name = self.patch_select_selected_patch_name();
                self.open_patch_select_overlay(selected_patch_name.as_deref());
            }
            KeyCode::Enter if self.patch_select_filter_active => {
                self.patch_select_filter_active = false;
                self.sync_patch_select_states();
            }
            KeyCode::Enter => {
                if let Some(selected) = self.patch_select_selected_patch_name() {
                    self.replace_current_line_patch(&selected);
                    let line = self.lines[self.cursor].clone();
                    self.record_notepad_history(&line);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Left => {
                self.patch_select_focus = PatchSelectPane::Patches;
                self.sync_patch_select_states();
                self.preview_selected_patch();
            }
            KeyCode::Right => {
                self.patch_select_focus = PatchSelectPane::Favorites;
                self.sync_patch_select_states();
                self.preview_selected_patch();
            }
            KeyCode::Char('h') if !self.patch_select_filter_active => {
                self.patch_select_focus = PatchSelectPane::Patches;
                self.sync_patch_select_states();
                self.preview_selected_patch();
            }
            KeyCode::Char('l') if !self.patch_select_filter_active => {
                self.patch_select_focus = PatchSelectPane::Favorites;
                self.sync_patch_select_states();
                self.preview_selected_patch();
            }
            KeyCode::Char('j') if !self.patch_select_filter_active => {
                self.move_patch_select_selection_by(1);
            }
            KeyCode::Char('k') if !self.patch_select_filter_active => {
                self.move_patch_select_selection_by(-1);
            }
            KeyCode::Char('f') if !self.patch_select_filter_active => {
                let Some(patch_name) = self.patch_select_selected_patch_name() else {
                    return;
                };
                let Some(phrase) = self.patch_select_current_phrase() else {
                    return;
                };
                self.add_patch_phrase_favorite(patch_name, phrase);
                self.refresh_patch_select_favorites();
                self.sync_patch_select_states();
                self.preview_selected_patch();
            }
            KeyCode::Down => self.move_patch_select_selection_by(1),
            KeyCode::Up => self.move_patch_select_selection_by(-1),
            KeyCode::PageDown => {
                self.move_patch_select_selection_by(self.patch_select_page_size as isize)
            }
            KeyCode::PageUp => {
                self.move_patch_select_selection_by(-(self.patch_select_page_size as isize))
            }
            KeyCode::Char('/') => {
                self.patch_select_focus = PatchSelectPane::Patches;
                self.patch_select_filter_active = true;
                self.sync_patch_select_states();
            }
            KeyCode::Backspace if self.patch_select_filter_active => {
                if self.patch_query.pop().is_some() {
                    self.update_patch_filter();
                }
                if self.patch_query.is_empty() {
                    self.patch_select_filter_active = false;
                }
            }
            KeyCode::Char('?') => self.enter_help(),
            KeyCode::Char(c) if self.patch_select_filter_active => {
                self.patch_query.push(c);
                self.update_patch_filter();
            }
            _ => {}
        }
    }
}
