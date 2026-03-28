//! TUI のキー入力処理

use crossterm::event::{KeyCode, KeyModifiers};
use mmlabc_to_smf::mml_preprocessor;
use ratatui::widgets::ListState;
use std::time::{SystemTime, UNIX_EPOCH};
use tui_textarea::TextArea;

use super::{
    filter_patches, Mode, NormalAction, PatchLoadState, PlayState, TuiApp, PATCH_JSON_KEY,
};

const PATCH_SELECT_PREVIEW_FALLBACK_PHRASE: &str = "c";

impl<'a> TuiApp<'a> {
    fn build_patch_json(patch_name: &str) -> String {
        format!(
            "{{\"{PATCH_JSON_KEY}\": {}}}",
            serde_json::to_string(patch_name).unwrap_or_else(|_| format!("\"{}\"", patch_name))
        )
    }

    fn replace_current_line_patch(&mut self, patch_name: &str) {
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

    pub(super) fn play_mml(&mut self, mml: String) {
        #[cfg(test)]
        if self.entry_ptr == 0 {
            // new_for_test() では PluginEntry を持たないため、
            // テスト中は再生スレッドを起動せず play_state 更新だけを検証する。
            *self.play_state.lock().unwrap() = PlayState::Running(mml);
            return;
        }

        self.kick_play(mml);
    }

    fn play_current_line(&mut self) {
        let mml = self.lines[self.cursor].trim().to_string();
        if !mml.is_empty() {
            self.record_patch_phrase_history(&mml);
            self.play_mml(mml);
        }
    }

    fn pick_random_patch_name(&self) -> Result<String, String> {
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
        self.patch_cursor = self
            .lines
            .get(self.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .and_then(|(patch_name, _)| {
                self.patch_filtered
                    .iter()
                    .position(|patch| patch == &patch_name)
            })
            .unwrap_or(0);
        let mut ls = ListState::default();
        if !self.patch_filtered.is_empty() {
            ls.select(Some(self.patch_cursor));
        }
        self.patch_list_state = ls;
        self.mode = Mode::PatchSelect;
    }

    fn patch_select_preview_mml(&self) -> Option<String> {
        let patch_name = self.patch_filtered.get(self.patch_cursor)?;
        let line = self.lines.get(self.cursor)?;
        let preprocessed = mml_preprocessor::extract_embedded_json(line);
        let phrase = match preprocessed.remaining_mml.trim() {
            "" => PATCH_SELECT_PREVIEW_FALLBACK_PHRASE.to_string(),
            remaining => remaining.to_string(),
        };
        let json = Self::build_patch_json(patch_name);
        Some(format!("{json} {phrase}"))
    }

    fn preview_selected_patch(&mut self) {
        if let Some(mml) = self.patch_select_preview_mml() {
            self.play_mml(mml);
        }
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
                    self.replace_current_line_patch(&selected);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.patch_cursor + 1 < self.patch_filtered.len() {
                    self.patch_cursor += 1;
                    self.patch_list_state.select(Some(self.patch_cursor));
                    self.preview_selected_patch();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.patch_cursor > 0 {
                    self.patch_cursor -= 1;
                    self.patch_list_state.select(Some(self.patch_cursor));
                    self.preview_selected_patch();
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

    pub(super) fn handle_normal(&mut self, key: KeyCode) -> NormalAction {
        match key {
            KeyCode::Char('q') => return NormalAction::Quit,
            KeyCode::Char('d') => return NormalAction::LaunchDaw,
            KeyCode::Char('i') => self.start_insert(),
            KeyCode::Char('r') => match self.pick_random_patch_name() {
                Ok(patch_name) => {
                    self.replace_current_line_patch(&patch_name);
                    self.play_current_line();
                }
                Err(msg) => *self.play_state.lock().unwrap() = PlayState::Err(msg),
            },
            KeyCode::Char('t') => {
                if self.cfg.patches_dir.is_none() {
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
            KeyCode::Char('K') | KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Enter | KeyCode::Char(' ') => self.play_current_line(),
            _ => {}
        }
        NormalAction::Continue
    }

    pub(super) fn handle_insert(&mut self, key_event: crossterm::event::KeyEvent) {
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            match key_event.code {
                KeyCode::Char('c') => {
                    self.textarea.copy();
                    crate::clipboard::set_text(self.textarea.yank_text().to_string());
                    return;
                }
                KeyCode::Char('x') => {
                    self.textarea.cut();
                    return;
                }
                KeyCode::Char('v') => {
                    self.textarea.paste();
                    return;
                }
                _ => {}
            }
        }
        match key_event.code {
            KeyCode::Esc => {
                let text = self.textarea.lines().join("");
                self.lines[self.cursor] = text.clone();
                self.mode = Mode::Normal;
                if !text.trim().is_empty() {
                    self.record_patch_phrase_history(text.trim());
                    self.play_mml(text.trim().to_string());
                }
            }
            KeyCode::Enter => {
                // 確定 → 非同期再生 → 次行挿入 → INSERT 継続
                let text = self.textarea.lines().join("");
                self.lines[self.cursor] = text.clone();
                if !text.trim().is_empty() {
                    self.record_patch_phrase_history(text.trim());
                    self.play_mml(text.trim().to_string());
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
