//! TUI のキー入力処理

mod patch_select;

use super::{Mode, NormalAction, PlayState};
use crate::NotepadScreen;
use crossterm::event::{KeyCode, KeyModifiers};

impl<'a> NotepadScreen<'a> {
    /// notepad 画面のキー入力の入口。現在のサブモードへ振り分ける。
    ///
    /// 画面の切替や config 編集など、共有ランタイム側でしか行えない操作は
    /// [`NormalAction`] として呼び出し元（`tui::runtime`）へ返す。
    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> NormalAction {
        match self.mode {
            Mode::Normal => return self.handle_normal_key_event(key),
            Mode::Insert => self.handle_insert(key),
            Mode::PatchSelect => self.handle_patch_select(key),
            Mode::NotepadHistory => self.handle_notepad_history_key_event(key),
            Mode::PatchPhrase => self.handle_patch_phrase_key_event(key),
            Mode::NotepadHistoryGuide => self.handle_notepad_history_guide(key.code),
            Mode::Help => self.handle_help(key.code),
        }
        NormalAction::Continue
    }

    /// Ctrl+C は textarea 入力中のオーバーレイだけが処理する（コピー操作）。
    /// それ以外のモードでは何もせず、終了などに解釈しない。
    pub fn handle_ctrl_c(&mut self, key: crossterm::event::KeyEvent) {
        match self.mode {
            Mode::Insert => self.handle_insert(key),
            Mode::PatchSelect if self.patch_select.patch_select_filter_active => {
                self.handle_patch_select(key)
            }
            Mode::NotepadHistory if self.notepad_history.filter_active => {
                self.handle_notepad_history_key_event(key)
            }
            Mode::PatchPhrase if self.patch_phrase.filter_active => {
                self.handle_patch_phrase_key_event(key)
            }
            _ => {}
        }
    }

    pub(super) fn enter_help(&mut self) {
        self.help_origin = self.mode;
        self.mode = Mode::Help;
        cmrt_tui_core::memory::request_refresh();
    }

    fn set_normal_cursor(&mut self, next_cursor: usize) {
        self.set_normal_cursor_with_navigation_hint(next_cursor, None);
    }

    fn set_normal_cursor_with_navigation_hint(
        &mut self,
        next_cursor: usize,
        preferred_delta: Option<isize>,
    ) {
        if next_cursor != self.editor.cursor {
            self.editor.cursor = next_cursor;
            self.editor.list_state.select(Some(self.editor.cursor));
            self.play_current_line_with_navigation_hint(preferred_delta);
        }
    }

    fn move_normal_cursor_by(&mut self, delta: isize) {
        let max_cursor = self.editor.lines.len().saturating_sub(1) as isize;
        let next_cursor = (self.editor.cursor as isize + delta).clamp(0, max_cursor) as usize;
        self.set_normal_cursor_with_navigation_hint(next_cursor, Some(delta));
    }

    pub(super) fn handle_help(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            debug_assert_ne!(self.help_origin, Mode::Help);
            self.mode = self.help_origin;
        }
    }

    pub(super) fn handle_normal_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> NormalAction {
        if key_event.modifiers == KeyModifiers::NONE
            && matches!(key_event.code, KeyCode::Char('j' | 'k'))
        {
            self.sound_check_guide.complete();
        }
        if key_event.modifiers.contains(KeyModifiers::SHIFT) && key_event.code == KeyCode::Char('H')
        {
            self.editor.pending_delete = false;
            match self.current_line_patch_name() {
                Some(patch_name) => self.start_patch_phrase_for_patch_name(Some(patch_name)),
                None => self.start_notepad_history_guide(),
            }
            return NormalAction::Continue;
        }

        self.handle_normal(key_event.code)
    }

    fn start_notepad_history_guide(&mut self) {
        self.mode = Mode::NotepadHistoryGuide;
    }

    pub(super) fn handle_notepad_history_guide(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => self.start_notepad_history(),
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    pub fn handle_normal(&mut self, key: KeyCode) -> NormalAction {
        match key {
            KeyCode::Char('d') => {
                if self.editor.pending_delete {
                    self.editor.pending_delete = false;
                    self.delete_current_line();
                } else {
                    self.editor.pending_delete = true;
                }
            }
            _ => {
                self.editor.pending_delete = false;
                match key {
                    KeyCode::Char('q') => return NormalAction::Quit,
                    KeyCode::Char('w') => return NormalAction::LaunchDaw,
                    KeyCode::Char('v') => return NormalAction::LaunchKeyboard,
                    KeyCode::Char('e') => return NormalAction::EditConfig,
                    KeyCode::Char('b') => return NormalAction::LaunchLoopBrowser,
                    KeyCode::Char('i') => self.start_insert(),
                    KeyCode::Char('g') => match self.insert_generated_line_above() {
                        Ok(()) => {}
                        Err(msg) => {
                            *self.playback.session.play_state().lock().unwrap() =
                                PlayState::Err(msg)
                        }
                    },
                    KeyCode::Char('r') => {
                        let filter_query = self.current_line_random_patch_filter_query();
                        match self.pick_random_patch_name_with_query(filter_query.as_deref()) {
                            Ok(Some(patch_name)) => {
                                self.replace_current_line_patch_with_filter(
                                    &patch_name,
                                    filter_query.as_deref(),
                                );
                                self.play_current_line();
                            }
                            Ok(None) => {}
                            Err(msg) => {
                                *self.playback.session.play_state().lock().unwrap() =
                                    PlayState::Err(msg)
                            }
                        }
                    }
                    KeyCode::Char('t') => {
                        self.open_patch_select_overlay(None);
                    }
                    KeyCode::Char('p') if !self.paste_yanked_line(false) => {
                        self.set_empty_yank_error();
                    }
                    KeyCode::Char('P') if !self.paste_yanked_line(true) => {
                        self.set_empty_yank_error();
                    }
                    KeyCode::Char('f') => self.start_patch_phrase_for_current_line(),
                    KeyCode::Char('o') => {
                        self.insert_empty_line_and_start_insert(self.editor.cursor + 1);
                    }
                    KeyCode::Char('O') => {
                        self.insert_empty_line_and_start_insert(self.editor.cursor);
                    }
                    KeyCode::Delete => {
                        self.delete_current_line();
                    }
                    KeyCode::Char('j') | KeyCode::Down => self.move_normal_cursor_by(1),
                    KeyCode::Char('k') | KeyCode::Up => self.move_normal_cursor_by(-1),
                    KeyCode::PageDown => self.move_normal_cursor_by(self.editor.page_size as isize),
                    KeyCode::PageUp => {
                        self.move_normal_cursor_by(-(self.editor.page_size as isize))
                    }
                    KeyCode::Home => {
                        self.set_normal_cursor(0);
                    }
                    KeyCode::Char('M') => {
                        self.set_normal_cursor(self.editor.lines.len() / 2);
                    }
                    KeyCode::Char('K') | KeyCode::Char('?') => self.enter_help(),
                    KeyCode::Enter | KeyCode::Char(' ') => self.play_current_line(),
                    _ => {}
                }
            }
        }
        NormalAction::Continue
    }

    pub(super) fn handle_insert(&mut self, key_event: crossterm::event::KeyEvent) {
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            match key_event.code {
                KeyCode::Char('c') => {
                    self.editor.textarea.copy();
                    cmrt_tui_core::clipboard::set_text(
                        self.editor.textarea.yank_text().to_string(),
                    );
                    return;
                }
                KeyCode::Char('x') => {
                    self.editor.textarea.cut();
                    return;
                }
                KeyCode::Char('v') => {
                    self.editor.textarea.paste();
                    return;
                }
                _ => {}
            }
        }
        match key_event.code {
            KeyCode::Esc => {
                let text = self.editor.textarea.lines().join("");
                self.editor.lines[self.editor.cursor] = text.clone();
                self.mode = Mode::Normal;
                if !text.trim().is_empty() {
                    self.record_notepad_history(text.trim());
                    self.record_patch_phrase_history(text.trim());
                    self.play_mml(text.trim().to_string());
                }
            }
            KeyCode::Enter => {
                // 確定 → 非同期再生 → 次行挿入 → INSERT 継続
                let text = self.editor.textarea.lines().join("");
                self.editor.lines[self.editor.cursor] = text.clone();
                if !text.trim().is_empty() {
                    self.record_notepad_history(text.trim());
                    self.record_patch_phrase_history(text.trim());
                    self.play_mml(text.trim().to_string());
                }
                self.editor
                    .lines
                    .insert(self.editor.cursor + 1, String::new());
                self.editor.cursor += 1;
                self.editor.list_state.select(Some(self.editor.cursor));
                self.editor.textarea = cmrt_tui_core::text_input::new_single_line_textarea("");
            }
            _ => {
                self.editor.textarea.input(key_event);
            }
        }
    }
}
