//! DAW モードのキー入力処理

use crossterm::event::KeyCode;

use super::{
    playback_util::effective_measure_count, DawApp, DawMode, DawNormalAction, DawPlayState,
};

fn format_random_patch_hot_reload_log(
    track: usize,
    displayed_measure_index: Option<usize>,
    old_effective_count: Option<usize>,
    new_effective_count: Option<usize>,
    old_measure_samples: usize,
    new_measure_samples: usize,
) -> String {
    let displayed = displayed_measure_index
        .map(|measure_index| format!("meas{}", measure_index + 1))
        .unwrap_or_else(|| "none".to_string());
    format!(
        "play: hot reload random patch track{track} display={displayed} effective_count={old_effective_count:?}->{new_effective_count:?} measure_samples={old_measure_samples}->{new_measure_samples}"
    )
}

impl DawApp {
    // ─── INSERT モード ────────────────────────────────────────

    fn commit_insert_cell(&mut self, track: usize, measure: usize, text: &str) -> bool {
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

    pub(super) fn start_insert(&mut self) {
        let mut ta = tui_textarea::TextArea::default();
        for ch in self.data[self.cursor_track][self.cursor_measure].chars() {
            ta.insert_char(ch);
        }
        self.textarea = ta;
        self.mode = DawMode::Insert;
    }

    /// 編集内容を確定してキャッシュ更新・保存を行う
    pub(super) fn commit_insert(&mut self) {
        let text = self.textarea.lines().join("");
        let changed = self.commit_insert_cell(self.cursor_track, self.cursor_measure, &text);

        if !changed {
            return;
        }

        self.save();

        // hot reload: 次の再生ループから新しい MML と小節サンプル数を反映する
        // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
        // ロック取得前に実行する
        let new_mmls = self.build_measure_mmls();
        let new_samples = self.measure_duration_samples();
        *self.play_measure_mmls.lock().unwrap() = new_mmls;
        *self.play_measure_samples.lock().unwrap() = new_samples;
    }

    // ─── キー処理 ─────────────────────────────────────────────

    pub(super) fn handle_help(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.mode = DawMode::Normal;
        }
    }

    pub(super) fn handle_normal(&mut self, key: KeyCode) -> DawNormalAction {
        match key {
            KeyCode::Char('q') => return DawNormalAction::QuitApp,
            KeyCode::Char('d') | KeyCode::Esc => return DawNormalAction::ReturnToTui,

            KeyCode::Char('h') | KeyCode::Left => {
                if self.cursor_measure > 0 {
                    self.cursor_measure -= 1;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.cursor_measure < self.measures {
                    self.cursor_measure += 1;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor_track + 1 < self.tracks {
                    self.cursor_track += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor_track > 0 {
                    self.cursor_track -= 1;
                }
            }
            KeyCode::Char('H') => {
                self.cursor_track = 0;
            }
            KeyCode::Char('M') => {
                self.cursor_track = self.tracks / 2;
            }
            KeyCode::Char('L') => {
                self.cursor_track = self.tracks - 1;
            }

            KeyCode::Char('i') => self.start_insert(),

            KeyCode::Char('K') | KeyCode::Char('?') => self.mode = DawMode::Help,

            KeyCode::Char('p') => {
                let state = *self.play_state.lock().unwrap();
                if state == DawPlayState::Playing || state == DawPlayState::Preview {
                    self.stop_play();
                } else {
                    self.start_play();
                }
            }

            KeyCode::Char('r') => {
                // measure 0 にランダム音色を設定
                if let Some(patch) = self.pick_random_patch_name() {
                    let affected_measures: Vec<usize> = (1..=self.measures)
                        .filter(|&measure| !self.data[self.cursor_track][measure].trim().is_empty())
                        .collect();
                    self.data[self.cursor_track][0] =
                        format!("{{\"Surge XT patch\": \"{}\"}}", patch);
                    self.invalidate_cell(self.cursor_track, 0);
                    self.invalidate_dependent_cells(self.cursor_track, 0);
                    // 依存セルはまとめてキュー投入せず、次の再生小節を優先して 1 件ずつ予約する。
                    self.start_track_rerender_batch(
                        self.cursor_track,
                        &affected_measures,
                        "random patch update",
                    );
                    self.save();

                    // hot reload: 次の再生ループから新しい音色を反映する
                    // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
                    // ロック取得前に実行する
                    let new_mmls = self.build_measure_mmls();
                    let new_samples = self.measure_duration_samples();
                    let old_effective_count = {
                        let old_mmls = self.play_measure_mmls.lock().unwrap();
                        effective_measure_count(&old_mmls)
                    };
                    let new_effective_count = effective_measure_count(&new_mmls);
                    let old_samples = *self.play_measure_samples.lock().unwrap();
                    let displayed_measure_index = self
                        .play_position
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|position| position.measure_index);
                    self.append_log_line(format_random_patch_hot_reload_log(
                        self.cursor_track,
                        displayed_measure_index,
                        old_effective_count,
                        new_effective_count,
                        old_samples,
                        new_samples,
                    ));
                    *self.play_measure_mmls.lock().unwrap() = new_mmls;
                    *self.play_measure_samples.lock().unwrap() = new_samples;
                }
            }

            _ => {}
        }
        DawNormalAction::Continue
    }

    pub(super) fn handle_insert(&mut self, key_event: crossterm::event::KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                let confirmed_measure = self.cursor_measure;
                self.commit_insert();
                // 非演奏中の場合、確定した小節をプレビュー再生する
                if *self.play_state.lock().unwrap() == DawPlayState::Idle && confirmed_measure > 0 {
                    self.start_preview(confirmed_measure - 1);
                }
                self.mode = DawMode::Normal;
            }
            KeyCode::Enter => {
                // 確定 → 次の小節へ → INSERT 継続
                let confirmed_measure = self.cursor_measure;
                self.commit_insert();
                // 非演奏中の場合、確定した小節をプレビュー再生する
                if *self.play_state.lock().unwrap() == DawPlayState::Idle && confirmed_measure > 0 {
                    self.start_preview(confirmed_measure - 1);
                }
                if self.cursor_measure < self.measures {
                    self.cursor_measure += 1;
                }
                self.start_insert();
            }
            _ => {
                self.textarea.input(key_event);
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/daw/input.rs"]
mod tests;
