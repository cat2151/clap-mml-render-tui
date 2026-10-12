use crossterm::event::{KeyCode, KeyEvent};

use super::super::{DawApp, DawMode, DawPlayState};

impl DawApp {
    pub(crate) fn commit_insert_cell(&mut self, track: usize, measure: usize, text: &str) -> bool {
        if self.editor.data[track][measure] == text {
            return false;
        }
        self.editor.data[track][measure] = text.to_string();
        self.sync_http_grid_snapshot();
        self.invalidate_cell(track, measure);
        self.kick_cache(track, measure);
        // track0 または音色セル変更時は依存セルも再キャッシュする
        self.invalidate_and_kick_dependent_cells(track, measure);
        true
    }

    pub(crate) fn start_insert(&mut self) {
        self.textarea = cmrt_tui_core::text_input::new_single_line_textarea(
            &self.editor.data[self.editor.cursor_track][self.editor.cursor_measure],
        );
        self.mode = DawMode::Insert;
    }

    /// 編集内容を確定してキャッシュ更新・保存を行う
    pub(crate) fn commit_insert(&mut self) {
        let text = self.textarea.lines().join("");
        let changed =
            self.commit_insert_cell(self.editor.cursor_track, self.editor.cursor_measure, &text);

        if !changed {
            return;
        }

        self.save();

        // hot reload: 次の再生ループから新しい MML と小節サンプル数を反映する
        // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
        // ロック取得前に実行する
        self.sync_playback_mml_state();
    }

    pub(crate) fn handle_insert(&mut self, key_event: KeyEvent) {
        // セルは 1 行なので、複数行は先頭の 1 行だけを貼る（MML overlay の 1 行モードと同じ）。
        if cmrt_tui_core::clipboard::handle_textarea_key(&mut self.textarea, key_event, true) {
            return;
        }
        match key_event.code {
            KeyCode::Esc => {
                let confirmed_measure = self.editor.cursor_measure;
                self.commit_insert();
                if *self.playback.play_state.lock().unwrap() == DawPlayState::Idle
                    && confirmed_measure > 0
                {
                    self.start_preview(confirmed_measure - 1);
                }
                self.mode = DawMode::Normal;
            }
            KeyCode::Enter => {
                let confirmed_measure = self.editor.cursor_measure;
                self.commit_insert();
                if *self.playback.play_state.lock().unwrap() == DawPlayState::Idle
                    && confirmed_measure > 0
                {
                    self.start_preview(confirmed_measure - 1);
                }
                if self.editor.cursor_measure < self.editor.measures {
                    self.editor.cursor_measure += 1;
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
