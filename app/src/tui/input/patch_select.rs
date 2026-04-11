mod state;

use crossterm::event::{KeyCode, KeyModifiers};
use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tui::{
    filter_patches, PatchLoadState, PatchSelectPane, PATCH_FILTER_QUERY_JSON_KEY, PATCH_JSON_KEY,
};

use super::{Mode, PlayState, TuiApp};

const PATCH_SELECT_PREVIEW_FALLBACK_PHRASE: &str = "c";

impl<'a> TuiApp<'a> {
    fn resolve_loaded_patch_name(&self, patch_name: &str) -> Option<String> {
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Ready(pairs) => {
                crate::patches::resolve_display_patch_name(pairs, patch_name)
            }
            PatchLoadState::Loading | PatchLoadState::Err(_) => None,
        }
    }

    pub(in crate::tui) fn normalize_patch_phrase_store_key(
        &mut self,
        patch_name: String,
    ) -> String {
        let Some(resolved) = self.resolve_loaded_patch_name(&patch_name) else {
            return patch_name;
        };
        if resolved != patch_name
            && crate::history::rename_patch_phrase_store_key(
                &mut self.patch_phrase_store,
                &patch_name,
                &resolved,
            )
        {
            self.patch_phrase_store_dirty = true;
        }
        resolved
    }

    pub(in crate::tui) fn normalize_patch_phrase_store_for_available_patches(
        &mut self,
        pairs: &[(String, String)],
    ) {
        if crate::history::normalize_patch_phrase_store_for_available_patches(
            &mut self.patch_phrase_store,
            pairs,
        ) {
            self.patch_phrase_store_dirty = true;
        }
    }

    fn normalize_current_line_patch_json_if_known(&mut self) {
        let Some(current_patch_name) = self.current_line_patch_name() else {
            return;
        };
        let Some(raw_patch_name) = self
            .lines
            .get(self.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
        else {
            return;
        };
        if current_patch_name != raw_patch_name {
            self.replace_current_line_patch(&current_patch_name);
        }
    }

    fn build_patch_json(patch_name: &str) -> String {
        Self::build_patch_json_with_filter_query(patch_name, None)
    }

    fn build_patch_json_with_filter_query(patch_name: &str, filter_query: Option<&str>) -> String {
        let patch_name =
            serde_json::to_string(patch_name).unwrap_or_else(|_| format!("\"{}\"", patch_name));
        match filter_query
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            Some(filter_query) => {
                let filter_query = serde_json::to_string(filter_query)
                    .unwrap_or_else(|_| format!("\"{}\"", filter_query));
                format!(
                    r#"{{"{PATCH_JSON_KEY}": {patch_name}, "{PATCH_FILTER_QUERY_JSON_KEY}": {filter_query}}}"#
                )
            }
            None => format!(r#"{{"{PATCH_JSON_KEY}": {patch_name}}}"#),
        }
    }

    fn extract_patch_json_value(mml: &str) -> Option<Value> {
        let preprocessed = mml_preprocessor::extract_embedded_json(mml);
        preprocessed
            .embedded_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
    }

    fn patch_category_filter_query(patch_name: &str) -> Option<String> {
        let (parent, _) = patch_name.rsplit_once('/')?;
        let category = parent.rsplit('/').next()?.trim();
        if category.is_empty() {
            None
        } else {
            Some(category.to_lowercase())
        }
    }

    fn current_line_patch_filter_query(&self) -> Option<String> {
        self.lines.get(self.cursor).and_then(|line| {
            Self::extract_patch_json_value(line).and_then(|value| {
                value
                    .get(PATCH_FILTER_QUERY_JSON_KEY)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
        })
    }

    fn has_matching_patches_for_query(&self, query: &str) -> bool {
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Ready(pairs) => !filter_patches(pairs, query).is_empty(),
            PatchLoadState::Loading | PatchLoadState::Err(_) => false,
        }
    }

    fn current_line_random_patch_filter_query(&self) -> Option<String> {
        self.current_line_patch_filter_query()
            .and_then(|query| self.has_matching_patches_for_query(&query).then_some(query))
            .or_else(|| {
                self.current_line_patch_name()
                    .and_then(|patch_name| Self::patch_category_filter_query(&patch_name))
                    .filter(|query| self.has_matching_patches_for_query(query))
            })
    }

    pub(super) fn replace_current_line_patch(&mut self, patch_name: &str) {
        let filter_query = self.current_line_patch_filter_query();
        self.replace_current_line_patch_with_filter(patch_name, filter_query.as_deref());
    }

    pub(super) fn replace_current_line_patch_with_filter(
        &mut self,
        patch_name: &str,
        filter_query: Option<&str>,
    ) {
        let json = Self::build_patch_json_with_filter_query(patch_name, filter_query);
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
        self.normalize_current_line_patch_json_if_known();
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
        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            return Err("patches_dirs が設定されていません".to_string());
        }
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
            PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
            PatchLoadState::Ready(pairs) if pairs.is_empty() => {
                Err("patches_dirs にパッチが見つかりません".to_string())
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

    fn pick_random_patch_name_with_query(
        &self,
        query: Option<&str>,
    ) -> Result<Option<String>, String> {
        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            return Err("patches_dirs が設定されていません".to_string());
        }
        let state = self.patch_load_state.lock().unwrap();
        match &*state {
            PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
            PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
            PatchLoadState::Ready(pairs) if pairs.is_empty() => {
                Err("patches_dirs にパッチが見つかりません".to_string())
            }
            PatchLoadState::Ready(pairs) => {
                let filtered = match query {
                    Some(query) => {
                        let filtered = filter_patches(pairs, query);
                        if filtered.is_empty() {
                            return Ok(None);
                        }
                        filtered
                    }
                    None => pairs.iter().map(|(orig, _)| orig.clone()).collect(),
                };
                let ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0);
                let index = (ns % filtered.len() as u128) as usize;
                Ok(Some(filtered[index].clone()))
            }
        }
    }

    pub(super) fn pick_random_patch_for_current_line(
        &self,
    ) -> Result<Option<(String, Option<String>)>, String> {
        let filter_query = self.current_line_random_patch_filter_query();
        self.pick_random_patch_name_with_query(filter_query.as_deref())
            .map(|patch_name| patch_name.map(|patch_name| (patch_name, filter_query)))
    }

    pub(in crate::tui) fn start_insert(&mut self) {
        self.textarea = crate::text_input::new_single_line_textarea(&self.lines[self.cursor]);
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
            .map(|patch_name| {
                self.resolve_loaded_patch_name(&patch_name)
                    .unwrap_or(patch_name)
            })
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

    pub(in crate::tui) fn start_patch_select(&mut self) {
        self.start_patch_select_with_initial_patch_name(None);
    }

    pub(in crate::tui) fn open_patch_select_overlay(&mut self, initial_patch_name: Option<&str>) {
        if !crate::patches::has_configured_patch_dirs(&self.cfg) {
            *self.play_state.lock().unwrap() =
                PlayState::Err("patches_dirs が設定されていません".to_string());
            return;
        }

        let action = {
            let state = self.patch_load_state.lock().unwrap();
            match &*state {
                PatchLoadState::Loading => Err("パッチを読み込み中です...".to_string()),
                PatchLoadState::Err(e) => Err(format!("パッチの読み込みに失敗: {}", e)),
                PatchLoadState::Ready(p) if p.is_empty() => {
                    Err("patches_dirs にパッチが見つかりません".to_string())
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

        if self.patch_select_filter_active {
            crate::text_input::sync_single_line_textarea(
                &mut self.patch_query_textarea,
                &self.patch_query,
            );
            match key_event.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    self.patch_select_filter_active = false;
                    self.sync_patch_select_states();
                }
                KeyCode::Backspace if self.patch_query.is_empty() => {
                    self.patch_select_filter_active = false;
                }
                KeyCode::Char('?') => self.enter_help(),
                _ => {
                    let previous_query = self.patch_query.clone();
                    if self.patch_query_textarea.input(key_event) {
                        let next_query =
                            crate::text_input::textarea_value(&self.patch_query_textarea);
                        if next_query == previous_query {
                            return;
                        }
                        self.patch_query = next_query;
                        self.update_patch_filter();
                        if !previous_query.is_empty() && self.patch_query.is_empty() {
                            self.patch_select_filter_active = false;
                        }
                    }
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
                self.patch_query_textarea =
                    crate::text_input::new_single_line_textarea(&self.patch_query);
                self.sync_patch_select_states();
            }
            KeyCode::Char('?') => self.enter_help(),
            _ => {}
        }
    }
}
