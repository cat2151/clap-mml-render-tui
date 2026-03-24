//! TUI のキー入力処理

use crossterm::event::KeyCode;
use mmlabc_to_smf::mml_preprocessor;
use ratatui::widgets::ListState;
use serde_json::Value;
use tui_textarea::TextArea;

use super::{
    filter_patches, Mode, NormalAction, PatchLoadState, PatchPhrasePane, PlayState, TuiApp,
};

const PATCH_JSON_KEY: &str = "Surge XT patch";
const PATCH_PHRASE_LIST_MAX_LEN: usize = 100;

impl<'a> TuiApp<'a> {
    fn push_front_dedup(items: &mut Vec<String>, item: String) {
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
        self.patch_phrase_name
            .as_deref()
            .and_then(|patch| self.patch_phrase_store.patches.get(patch))
            .map(|state| state.history.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| vec!["c".to_string()])
    }

    pub(super) fn patch_phrase_favorite_items(&self) -> Vec<String> {
        self.patch_phrase_name
            .as_deref()
            .and_then(|patch| self.patch_phrase_store.patches.get(patch))
            .map(|state| state.favorites.clone())
            .filter(|items| !items.is_empty())
            .unwrap_or_else(|| vec!["c".to_string()])
    }

    fn sync_patch_phrase_states(&mut self) {
        let history_len = self.patch_phrase_history_items().len();
        self.patch_phrase_history_cursor = self.patch_phrase_history_cursor.min(history_len - 1);
        self.patch_phrase_history_state
            .select(Some(self.patch_phrase_history_cursor));

        let favorites_len = self.patch_phrase_favorite_items().len();
        self.patch_phrase_favorites_cursor =
            self.patch_phrase_favorites_cursor.min(favorites_len - 1);
        self.patch_phrase_favorites_state
            .select(Some(self.patch_phrase_favorites_cursor));
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

        let state = self
            .patch_phrase_store
            .patches
            .entry(patch_name)
            .or_default();
        Self::push_front_dedup(&mut state.history, phrase);
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
        self.patch_phrase_name = Some(patch_name);
        self.patch_phrase_focus = PatchPhrasePane::History;
        self.patch_phrase_history_cursor = 0;
        self.patch_phrase_favorites_cursor = 0;
        self.sync_patch_phrase_states();
        self.mode = Mode::PatchPhrase;
    }

    pub(super) fn start_insert(&mut self) {
        self.textarea = TextArea::default();
        let current = self.lines[self.cursor].clone();
        for ch in current.chars() {
            self.textarea.insert_char(ch);
        }
        self.mode = Mode::Insert;
    }

    pub(super) fn start_patch_select(&mut self) {
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
        self.patch_cursor = 0;
        let mut ls = ListState::default();
        if !self.patch_filtered.is_empty() {
            ls.select(Some(0));
        }
        self.patch_list_state = ls;
        self.mode = Mode::PatchSelect;
    }

    pub(super) fn update_patch_filter(&mut self) {
        self.patch_filtered = filter_patches(&self.patch_all, &self.patch_query);
        self.patch_cursor = 0;
        if !self.patch_filtered.is_empty() {
            self.patch_list_state.select(Some(0));
        } else {
            self.patch_list_state.select(None);
        }
    }

    pub(super) fn handle_patch_select(&mut self, key_event: crossterm::event::KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                if !self.patch_filtered.is_empty() {
                    let selected = self.patch_filtered[self.patch_cursor].clone();
                    // serde_json を使って値を適切にエスケープする（パスに引用符・バックスラッシュが含まれる場合も安全）
                    let json = format!(
                        "{{\"{PATCH_JSON_KEY}\": {}}}",
                        serde_json::to_string(&selected)
                            .unwrap_or_else(|_| format!("\"{}\"", selected))
                    );
                    // 現在行の既存JSON（あれば）を除去して先頭に新しいJSONを挿入する
                    let current = self.lines[self.cursor].clone();
                    let preprocessed = mml_preprocessor::extract_embedded_json(&current);
                    let remaining = preprocessed.remaining_mml.trim().to_string();
                    self.lines[self.cursor] = if remaining.is_empty() {
                        json
                    } else {
                        format!("{} {}", json, remaining)
                    };
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Down => {
                if self.patch_cursor + 1 < self.patch_filtered.len() {
                    self.patch_cursor += 1;
                    self.patch_list_state.select(Some(self.patch_cursor));
                }
            }
            KeyCode::Up => {
                if self.patch_cursor > 0 {
                    self.patch_cursor -= 1;
                    self.patch_list_state.select(Some(self.patch_cursor));
                }
            }
            KeyCode::Backspace => {
                self.patch_query.pop();
                self.update_patch_filter();
            }
            KeyCode::Char(c) => {
                self.patch_query.push(c);
                self.update_patch_filter();
            }
            _ => {}
        }
    }

    pub(super) fn handle_help(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.mode = Mode::Normal;
        }
    }

    pub(super) fn handle_patch_phrase(&mut self, key: KeyCode) {
        let history_len = self.patch_phrase_history_items().len();
        let favorites_len = self.patch_phrase_favorite_items().len();

        match key {
            KeyCode::Esc => {
                self.flush_patch_phrase_store_if_dirty();
                self.mode = Mode::Normal;
            }
            KeyCode::Char('h') => {
                self.patch_phrase_focus = PatchPhrasePane::History;
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.kick_play(mml);
                }
            }
            KeyCode::Char('l') => {
                self.patch_phrase_focus = PatchPhrasePane::Favorites;
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.kick_play(mml);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.patch_phrase_focus {
                    PatchPhrasePane::History
                        if self.patch_phrase_history_cursor + 1 < history_len =>
                    {
                        self.patch_phrase_history_cursor += 1;
                    }
                    PatchPhrasePane::Favorites
                        if self.patch_phrase_favorites_cursor + 1 < favorites_len =>
                    {
                        self.patch_phrase_favorites_cursor += 1;
                    }
                    _ => {}
                }
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.kick_play(mml);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.patch_phrase_focus {
                    PatchPhrasePane::History if self.patch_phrase_history_cursor > 0 => {
                        self.patch_phrase_history_cursor -= 1;
                    }
                    PatchPhrasePane::Favorites if self.patch_phrase_favorites_cursor > 0 => {
                        self.patch_phrase_favorites_cursor -= 1;
                    }
                    _ => {}
                }
                self.sync_patch_phrase_states();
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.kick_play(mml);
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
                let state = self
                    .patch_phrase_store
                    .patches
                    .entry(patch_name)
                    .or_default();
                Self::push_front_dedup(&mut state.favorites, phrase);
                self.patch_phrase_focus = PatchPhrasePane::Favorites;
                self.patch_phrase_favorites_cursor = 0;
                self.sync_patch_phrase_states();
                self.patch_phrase_store_dirty = true;
                if let Some(mml) = self.patch_phrase_preview_mml() {
                    self.kick_play(mml);
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_normal(&mut self, key: KeyCode) -> NormalAction {
        match key {
            KeyCode::Char('q') => return NormalAction::Quit,
            KeyCode::Char('d') => return NormalAction::LaunchDaw,
            KeyCode::Char('i') => self.start_insert(),
            KeyCode::Char('r') => {
                self.random_timbre_enabled = !self.random_timbre_enabled;
            }
            KeyCode::Char('t') => {
                if self.random_timbre_enabled {
                    *self.play_state.lock().unwrap() =
                        PlayState::Err("ランダム音色モードでは音色選択は使えません".to_string());
                } else if self.cfg.patches_dir.is_none() {
                    *self.play_state.lock().unwrap() =
                        PlayState::Err("patches_dir が設定されていません".to_string());
                } else {
                    // バックグラウンドロードの状態を確認する
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
                        Ok(()) => self.start_patch_select(),
                    }
                }
            }
            KeyCode::Char('p') => {
                let current = self.lines[self.cursor].clone();
                match Self::extract_patch_phrase(&current) {
                    Some((patch_name, _)) => self.start_patch_phrase(patch_name),
                    None => {
                        *self.play_state.lock().unwrap() = PlayState::Err(
                            "現在行の先頭に patch name JSON がありません".to_string(),
                        );
                    }
                }
            }
            KeyCode::Char('o') => {
                self.lines.insert(self.cursor + 1, String::new());
                self.cursor += 1;
                self.list_state.select(Some(self.cursor));
                self.start_insert();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor + 1 < self.lines.len() {
                    self.cursor += 1;
                    self.list_state.select(Some(self.cursor));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.list_state.select(Some(self.cursor));
                }
            }
            KeyCode::Char('H') => {
                self.cursor = 0;
                self.list_state.select(Some(self.cursor));
            }
            KeyCode::Char('M') => {
                self.cursor = self.lines.len() / 2;
                self.list_state.select(Some(self.cursor));
            }
            KeyCode::Char('L') => {
                self.cursor = self.lines.len().saturating_sub(1);
                self.list_state.select(Some(self.cursor));
            }
            KeyCode::Char('K') => self.mode = Mode::Help,
            KeyCode::Enter | KeyCode::Char(' ') => {
                let mml = self.lines[self.cursor].trim().to_string();
                if !mml.is_empty() {
                    self.record_patch_phrase_history(&mml);
                    self.kick_play(mml);
                }
            }
            _ => {}
        }
        NormalAction::Continue
    }

    pub(super) fn handle_insert(&mut self, key_event: crossterm::event::KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                let text = self.textarea.lines().join("");
                self.lines[self.cursor] = text.clone();
                self.mode = Mode::Normal;
                if !text.trim().is_empty() {
                    self.record_patch_phrase_history(text.trim());
                    self.kick_play(text.trim().to_string());
                }
            }
            KeyCode::Enter => {
                // 確定 → 非同期再生 → 次行挿入 → INSERT 継続
                let text = self.textarea.lines().join("");
                self.lines[self.cursor] = text.clone();
                if !text.trim().is_empty() {
                    self.record_patch_phrase_history(text.trim());
                    self.kick_play(text.trim().to_string());
                }
                self.lines.insert(self.cursor + 1, String::new());
                self.cursor += 1;
                self.list_state.select(Some(self.cursor));
                self.textarea = TextArea::default();
            }
            _ => {
                self.textarea.input(key_event);
            }
        }
    }
}
