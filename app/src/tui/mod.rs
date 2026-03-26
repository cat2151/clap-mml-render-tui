//! vim 風 TUI
//!
//! モード:
//!   NORMAL : j/k で行移動、H/M/L で先頭/中央/末尾行へ移動、i/o で INSERT、r で現在行の先頭にランダム音色を挿入/置換、t で音色選択、Enter/Space で再生、q で終了
//!   INSERT : tui-textarea で編集
//!            ESC   → 確定 → NORMAL（再生開始）
//!            Enter → 確定 → 次行に新規行挿入 → INSERT 継続
//!            Ctrl+C / Ctrl+X / Ctrl+V → コピー / カット / ペースト
//!   PATCHSELECT : インクリメンタルサーチで音色を選択
//!            文字入力: フィルタ（space=AND条件）
//!            ↑↓:リスト移動  Enter:現在行の先頭にJSONで挿入（上書き）  ESC:キャンセル
//!   HELP : K / ? で表示、ESC でキャンセル

mod cache;
mod input;
mod patch_phrase;
mod playback_session;
mod ui;

use anyhow::Result;
use clack_host::prelude::PluginEntry;
use cmrt_core::{collect_patches, mml_render, to_relative, CoreConfig};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Frame, Terminal};
use tui_textarea::TextArea;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// audio_cache の最大エントリ数。超過時はキャッシュ全体をクリアしてから挿入する。
const AUDIO_CACHE_MAX_ENTRIES: usize = 64;
pub(super) const PATCH_JSON_KEY: &str = "Surge XT patch";

use self::cache::{filter_patches, resolve_cached_samples, try_insert_cache};
use crate::config::Config;

/// バックグラウンドパッチ読み込みの状態
enum PatchLoadState {
    Loading,
    Ready(Vec<(String, String)>), // (表示名, 小文字化済み表示名)
    Err(String),
}

#[derive(PartialEq)]
pub(super) enum Mode {
    Normal,
    Insert,
    PatchSelect,
    PatchPhrase,
    Help,
}

/// handle_normal の戻り値
enum NormalAction {
    Continue,
    Quit,
    LaunchDaw,
}

#[derive(Clone, PartialEq)]
pub(super) enum PlayState {
    Idle,
    Running(String), // レンダリング中
    Playing(String), // 演奏中
    Done(String),
    Err(String),
}

pub struct TuiApp<'a> {
    pub(super) mode: Mode,
    pub(super) lines: Vec<String>,
    pub(super) cursor: usize,
    pub(super) list_state: ListState,
    pub(super) textarea: TextArea<'a>,
    cfg: Arc<Config>,
    entry_ptr: usize, // *const PluginEntry as usize (main() に生存保証)
    pub(super) play_state: Arc<Mutex<PlayState>>,
    playback_session: Arc<AtomicU64>,
    active_sink: Arc<Mutex<Option<Arc<rodio::Sink>>>>,
    /// MML文字列 → レンダリング済みサンプルのキャッシュ
    pub(super) audio_cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    // 音色選択モード用
    /// バックグラウンドスレッドが収集したパッチリストの状態
    patch_load_state: Arc<Mutex<PatchLoadState>>,
    /// PatchSelect 起動時にスナップショットした (表示名, 小文字化済み) ペアのリスト
    pub(super) patch_all: Vec<(String, String)>,
    pub(super) patch_query: String,         // 検索クエリ
    pub(super) patch_filtered: Vec<String>, // フィルタ結果（表示名のみ）
    pub(super) patch_cursor: usize,         // フィルタ結果内のカーソル位置
    pub(super) patch_list_state: ListState, // 音色選択リスト描画用
    pub(super) patch_phrase_store: crate::history::PatchPhraseStore,
    pub(super) patch_phrase_name: Option<String>,
    pub(super) patch_phrase_history_cursor: usize,
    pub(super) patch_phrase_favorites_cursor: usize,
    pub(super) patch_phrase_history_state: ListState,
    pub(super) patch_phrase_favorites_state: ListState,
    pub(super) patch_phrase_focus: PatchPhrasePane,
    pub(super) patch_phrase_store_dirty: bool,
    /// バックグラウンドのアップデートチェックがtrueにセットしたらアップデートを実行
    pub update_available: Arc<AtomicBool>,
    /// 終了時 DAW モードだったかどうか（history.json に保存・復元する）
    pub(super) is_daw_mode: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PatchPhrasePane {
    History,
    Favorites,
}

impl<'a> TuiApp<'a> {
    pub fn new(cfg: &'a Config, entry: &'a PluginEntry) -> Self {
        let cfg_arc = Arc::new(cfg.clone());

        // パッチリストはバックグラウンドスレッドで収集する。
        // 起動時の同期スキャンによる遅延を避けるため。
        let patch_load_state: Arc<Mutex<PatchLoadState>> =
            Arc::new(Mutex::new(PatchLoadState::Loading));
        {
            let state_bg = Arc::clone(&patch_load_state);
            let patches_dir = cfg.patches_dir.clone();
            std::thread::spawn(move || match patches_dir {
                None => {
                    *state_bg.lock().unwrap() = PatchLoadState::Ready(Vec::new());
                }
                Some(dir) => match collect_patches(&dir) {
                    Ok(paths) => {
                        let pairs: Vec<(String, String)> = paths
                            .into_iter()
                            .map(|p| {
                                let rel = to_relative(&dir, &p);
                                let lower = rel.to_lowercase();
                                (rel, lower)
                            })
                            .collect();
                        *state_bg.lock().unwrap() = PatchLoadState::Ready(pairs);
                    }
                    Err(e) => {
                        *state_bg.lock().unwrap() = PatchLoadState::Err(e.to_string());
                    }
                },
            });
        }

        // `lines` は常に1行以上を保持する（不変条件）。
        // load_session_state() は lines が空でないことを保証している。
        let crate::history::SessionState {
            cursor,
            lines,
            is_daw_mode,
        } = crate::history::load_session_state();
        let initial_cursor = cursor.min(lines.len() - 1);
        let mut list_state = ListState::default();
        list_state.select(Some(initial_cursor));

        Self {
            mode: Mode::Normal,
            lines,
            cursor: initial_cursor,
            list_state,
            textarea: TextArea::default(),
            cfg: cfg_arc,
            entry_ptr: entry as *const PluginEntry as usize,
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            playback_session: Arc::new(AtomicU64::new(0)),
            active_sink: Arc::new(Mutex::new(None)),
            audio_cache: Arc::new(Mutex::new(HashMap::new())),
            patch_load_state,
            patch_all: Vec::new(),
            patch_query: String::new(),
            patch_filtered: Vec::new(),
            patch_cursor: 0,
            patch_list_state: ListState::default(),
            patch_phrase_store: crate::history::load_patch_phrase_store(),
            patch_phrase_name: None,
            patch_phrase_history_cursor: 0,
            patch_phrase_favorites_cursor: 0,
            patch_phrase_history_state: ListState::default(),
            patch_phrase_favorites_state: ListState::default(),
            patch_phrase_focus: PatchPhrasePane::History,
            patch_phrase_store_dirty: false,
            update_available: Arc::new(AtomicBool::new(false)),
            is_daw_mode,
        }
    }

    fn kick_play(&self, mml: String) {
        let cfg = Arc::clone(&self.cfg);
        let state = Arc::clone(&self.play_state);
        let playback_session = Arc::clone(&self.playback_session);
        let active_sink = Arc::clone(&self.active_sink);
        let cache = Arc::clone(&self.audio_cache);
        let entry_ptr = self.entry_ptr;
        let session = self.begin_playback_session();

        let cache_guard = cache.lock().unwrap();
        let cached_samples = resolve_cached_samples(Some(&cache_guard), &mml);
        drop(cache_guard);

        if let Some(samples) = cached_samples {
            // キャッシュヒット: レンダリングをスキップして即時再生
            let msg = format!("(cached) | {}", mml);
            self.set_play_state_if_current(session, PlayState::Playing(msg.clone()));

            std::thread::spawn(move || {
                Self::play_samples_for_session(
                    &state,
                    &playback_session,
                    &active_sink,
                    session,
                    cfg.sample_rate as u32,
                    samples,
                    msg,
                );
            });
        } else {
            // キャッシュミス: レンダリングが必要
            self.set_play_state_if_current(session, PlayState::Running(mml.clone()));

            std::thread::spawn(move || {
                // SAFETY: entry は main() のスタックに生存している
                let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };

                // レンダリング
                let core_cfg = CoreConfig::from(cfg.as_ref());
                let render_result = mml_render(&mml, &core_cfg, entry_ref);

                match render_result {
                    Err(e) => {
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err(format!("エラー: {}", e)),
                        );
                    }
                    Ok((samples, patch_name)) => {
                        if !Self::playback_session_is_current(&playback_session, session) {
                            return;
                        }
                        try_insert_cache(
                            &mut cache.lock().unwrap(),
                            mml.clone(),
                            samples.clone(),
                            false,
                        );

                        let msg = format!("{} | {}", patch_name, mml);
                        // 演奏中に切り替え
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Playing(msg.clone()),
                        );
                        Self::play_samples_for_session(
                            &state,
                            &playback_session,
                            &active_sink,
                            session,
                            cfg.sample_rate as u32,
                            samples,
                            msg,
                        );
                    }
                }
            });
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        ui::draw(self, f);
    }

    fn save_history_state(&self) {
        let _ = crate::history::save_session_state(&crate::history::SessionState {
            cursor: self.cursor,
            lines: self.lines.clone(),
            is_daw_mode: self.is_daw_mode,
        });
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // 前回 DAW モードで終了していた場合は直接 DAW モードで起動する
        let mut quit_from_startup_daw = false;
        if self.is_daw_mode {
            let mut daw = crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
            match daw.run_with_terminal(&mut terminal)? {
                crate::daw::DawExitReason::ReturnToTui => {
                    self.is_daw_mode = false;
                }
                crate::daw::DawExitReason::QuitApp => {
                    quit_from_startup_daw = true;
                }
            }
        }

        loop {
            if quit_from_startup_daw {
                break;
            }
            terminal.draw(|f| self.draw(f))?;

            // アップデートが利用可能になったら自動的にループを抜けてアップデートを実行する
            if self.update_available.load(Ordering::Relaxed) && self.mode == Mode::Normal {
                break;
            }

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    // Press のみ処理。Release/Repeat は無視（二重発火防止）
                    use crossterm::event::KeyEventKind;
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match self.mode {
                        Mode::Normal => match self.handle_normal(key.code) {
                            NormalAction::Quit => break,
                            NormalAction::LaunchDaw => {
                                self.flush_patch_phrase_store_if_dirty();
                                self.save_history_state();
                                let mut daw =
                                    crate::daw::DawApp::new(Arc::clone(&self.cfg), self.entry_ptr);
                                match daw.run_with_terminal(&mut terminal)? {
                                    crate::daw::DawExitReason::ReturnToTui => {
                                        self.is_daw_mode = false;
                                    }
                                    crate::daw::DawExitReason::QuitApp => {
                                        self.is_daw_mode = true;
                                        break;
                                    }
                                }
                            }
                            NormalAction::Continue => {}
                        },
                        Mode::Insert => self.handle_insert(key),
                        Mode::PatchSelect => self.handle_patch_select(key),
                        Mode::PatchPhrase => self.handle_patch_phrase(key.code),
                        Mode::Help => self.handle_help(key.code),
                    }
                }
            }
        }

        // 終了前にセッション状態を保存する（端末クリーンアップの成否に関わらず実行）。
        // 保存失敗はベストエフォートとして無視する（終了処理のため通知手段がない）。
        self.flush_patch_phrase_store_if_dirty();
        self.save_history_state();

        let raw_mode_result = disable_raw_mode();
        let alternate_screen_result = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        raw_mode_result?;
        alternate_screen_result?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/tui_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "../tests/tui.rs"]
mod tests;
