//! DAW モードのキー入力処理

use crossterm::event::KeyCode;

use super::{DawApp, DawMode, DawNormalAction, DawPlayState};

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

            KeyCode::Char('K') => self.mode = DawMode::Help,

            KeyCode::Char('p') => {
                let state = self.play_state.lock().unwrap().clone();
                if state == DawPlayState::Playing || state == DawPlayState::Preview {
                    self.stop_play();
                } else {
                    self.start_play();
                }
            }

            KeyCode::Char('r') => {
                // measure 0 にランダム音色を設定
                if let Some(patch) = self.pick_random_patch_name() {
                    self.data[self.cursor_track][0] =
                        format!("{{\"Surge XT patch\": \"{}\"}}", patch);
                    self.invalidate_cell(self.cursor_track, 0);
                    self.kick_cache(self.cursor_track, 0);
                    // 音色変更は当該 track の全小節（1..=MEASURES）に影響するため一括再キャッシュ（issue #67 参照）
                    self.invalidate_and_kick_dependent_cells(self.cursor_track, 0);
                    self.save();

                    // hot reload: 次の再生ループから新しい音色を反映する
                    // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
                    // ロック取得前に実行する
                    let new_mmls = self.build_measure_mmls();
                    let new_samples = self.measure_duration_samples();
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
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use tui_textarea::TextArea;

    use crate::config::Config;

    use super::super::{CacheState, CellCache, DawApp, DawMode, DawPlayState};

    fn build_test_app() -> (DawApp, std::sync::mpsc::Receiver<super::super::CacheJob>) {
        let tracks = 3;
        let measures = 2;
        let (cache_tx, cache_rx) = std::sync::mpsc::channel();
        (
            DawApp {
                data: vec![vec![String::new(); measures + 1]; tracks],
                cursor_track: 1,
                cursor_measure: 1,
                mode: DawMode::Normal,
                textarea: TextArea::default(),
                cfg: Arc::new(Config {
                    plugin_path: String::new(),
                    input_midi: String::new(),
                    output_midi: String::new(),
                    output_wav: String::new(),
                    sample_rate: 44_100.0,
                    buffer_size: 512,
                    patch_path: None,
                    patches_dir: None,
                    daw_tracks: tracks,
                    daw_measures: measures,
                }),
                entry_ptr: 0,
                tracks,
                measures,
                cache: Arc::new(Mutex::new(vec![
                    vec![CellCache::empty(); measures + 1];
                    tracks
                ])),
                cache_tx,
                render_lock: Arc::new(Mutex::new(())),
                play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
                play_position: Arc::new(Mutex::new(None)),
                play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
                play_measure_samples: Arc::new(Mutex::new(0)),
                log_lines: Arc::new(Mutex::new(VecDeque::new())),
            },
            cache_rx,
        )
    }

    #[test]
    fn commit_insert_skips_cache_refresh_when_text_is_unchanged() {
        let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_skips_cache_refresh");
        std::fs::remove_dir_all(&tmp).ok();

        {
            let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

            let (mut app, cache_rx) = build_test_app();
            app.data[1][1] = "cdef".to_string();
            {
                let mut cache = app.cache.lock().unwrap();
                cache[1][1].state = CacheState::Ready;
                cache[1][1].generation = 7;
            }

            app.start_insert();
            app.commit_insert();

            let cache = app.cache.lock().unwrap();
            assert_eq!(app.data[1][1], "cdef");
            assert!(matches!(cache[1][1].state, CacheState::Ready));
            assert_eq!(cache[1][1].generation, 7);
            assert!(
                cache_rx.try_recv().is_err(),
                "unchanged insert queued a cache job"
            );
        }

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn commit_insert_triggers_cache_refresh_when_text_changes() {
        let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_refreshes_cache");
        std::fs::remove_dir_all(&tmp).ok();

        {
            let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

            let (mut app, cache_rx) = build_test_app();
            app.data[1][1] = "cdef".to_string();
            {
                let mut cache = app.cache.lock().unwrap();
                cache[1][1].state = CacheState::Ready;
                cache[1][1].generation = 7;
            }

            app.start_insert();
            app.textarea = TextArea::default();
            for ch in "gfed".chars() {
                app.textarea.insert_char(ch);
            }
            app.commit_insert();

            let cache = app.cache.lock().unwrap();
            assert_eq!(app.data[1][1], "gfed");
            assert!(matches!(cache[1][1].state, CacheState::Pending));
            assert_eq!(cache[1][1].generation, 8);

            let job = cache_rx
                .try_recv()
                .expect("changed insert did not queue a cache job");
            assert_eq!(job.track, 1);
            assert_eq!(job.measure, 1);
            assert_eq!(job.generation, 8);
        }

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn commit_insert_keeps_semicolon_text_in_same_measure() {
        let tmp = std::env::temp_dir().join("cmrt_test_commit_insert_keeps_semicolon_text");
        std::fs::remove_dir_all(&tmp).ok();

        {
            let _guard = crate::test_utils::TestEnvGuard::set("CMRT_BASE_DIR", &tmp);

            let (mut app, cache_rx) = build_test_app();
            app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
            app.data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
            app.data[2][1] = "existing".to_string();

            app.start_insert();
            app.textarea = TextArea::default();
            for ch in "cde;gab".chars() {
                app.textarea.insert_char(ch);
            }
            app.commit_insert();

            assert_eq!(app.data[1][1], "cde;gab");
            assert_eq!(app.data[2][1], "existing");

            let job = cache_rx
                .try_recv()
                .expect("semicolon insert did not queue a cache job");
            assert_eq!(job.track, 1);
            assert_eq!(job.measure, 1);
            assert_eq!(
                job.mml.matches(r#"{"Surge XT patch": "piano"}"#).count(),
                2,
                "semicolon-separated phrases should each receive the track timbre: {}",
                job.mml
            );
            assert_eq!(
                job.mml.matches("t120").count(),
                2,
                "semicolon-separated phrases should each receive the track0/header content (t120): {}",
                job.mml
            );
            assert!(
                cache_rx.try_recv().is_err(),
                "unexpected extra cache job queued"
            );
        }

        std::fs::remove_dir_all(&tmp).ok();
    }
}
