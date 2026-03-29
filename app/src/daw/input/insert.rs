use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{DawApp, DawMode, DawPlayState};

impl DawApp {
    pub(super) fn commit_insert_cell(&mut self, track: usize, measure: usize, text: &str) -> bool {
        if self.data[track][measure] == text {
            return false;
        }
        self.data[track][measure] = text.to_string();
        self.invalidate_cell(track, measure);
        self.kick_cache(track, measure);
        // track0 または音色セル変更時は依存セルも再キャッシュする
        self.invalidate_and_kick_dependent_cells(track, measure);
        true
    }

    pub(in crate::daw) fn start_insert(&mut self) {
        let mut ta = tui_textarea::TextArea::default();
        for ch in self.data[self.cursor_track][self.cursor_measure].chars() {
            ta.insert_char(ch);
        }
        self.textarea = ta;
        self.mode = DawMode::Insert;
    }

    /// 編集内容を確定してキャッシュ更新・保存を行う
    pub(in crate::daw) fn commit_insert(&mut self) {
        let text = self.textarea.lines().join("");
        let changed = self.commit_insert_cell(self.cursor_track, self.cursor_measure, &text);

        if !changed {
            return;
        }

        self.save();

        // hot reload: 次の再生ループから新しい MML と小節サンプル数を反映する
        // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
        // ロック取得前に実行する
        self.sync_playback_mml_state();
    }

    pub(in crate::daw) fn handle_insert(&mut self, key_event: KeyEvent) {
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
                let confirmed_measure = self.cursor_measure;
                self.commit_insert();
                if *self.play_state.lock().unwrap() == DawPlayState::Idle && confirmed_measure > 0 {
                    self.start_preview(confirmed_measure - 1);
                }
                self.mode = DawMode::Normal;
            }
            KeyCode::Enter => {
                let confirmed_measure = self.cursor_measure;
                self.commit_insert();
                if *self.play_state.lock().unwrap() == DawPlayState::Idle && confirmed_measure > 0 {
                    self.start_preview(confirmed_measure - 1);
                }
                if self.cursor_measure < self.measures {
                    self.cursor_measure += 1;
                    self.update_ab_repeat_follow_end_with_cursor();
                }
                self.start_insert();
            }
            _ => {
                self.textarea.input(key_event);
            }
        }
    }
}
