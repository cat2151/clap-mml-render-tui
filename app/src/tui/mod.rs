//! vim 風 TUI
//!
//! モード:
//!   NORMAL : j/k で行移動、PageUp/PageDown で1画面移動、Home/M で先頭/中央行へ移動、i/o で INSERT、dd / Delete で現在行をヤンク削除、g で generate を現在行の上へ挿入して再生、r で現在行の先頭にランダム音色を挿入/置換、t で音色選択、Shift+H で patch history（patch name が無い場合は notepad history 案内）、f で patch history、w で DAW、Enter/Space で再生、q で終了
//!   INSERT : tui-textarea で編集
//!            ESC   → 確定 → NORMAL（再生開始）
//!            Enter → 確定 → 次行に新規行挿入 → INSERT 継続
//!            Ctrl+C / Ctrl+X / Ctrl+V → コピー / カット / ペースト
//!   PATCHSELECT : 音色を選択
//!            / の後に文字入力: patch name フィルタ（space=AND条件）
//!            n/p/t: notepad history / patch history / 音色選択
//!            j/k・↑↓・PageUp/PageDown:リスト移動（移動ごとにpreview再生）
//!            h/l・←/→:左右ペイン移動（移動ごとにpreview再生）
//!            Ctrl+S:sort順切替
//!            f:現在音色とMMLをFavorites追加
//!            Enter:現在行の先頭にJSONで挿入（上書き）  ESC:キャンセル
//!   HELP : K / ? で表示、ESC でキャンセル

mod cache;
mod input;
mod notepad_history;
mod patch_phrase;
mod playback_session;
mod prefetch;
mod render_queue;
mod runtime;
mod session;
mod ui;

use ratatui::{widgets::ListState, Frame};
use tui_textarea::TextArea;

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// audio_cache の最大エントリ数。超過時は古いエントリから1件ずつ退避する。
const AUDIO_CACHE_MAX_ENTRIES: usize = 64;
pub(super) const PATCH_JSON_KEY: &str = "Surge XT patch";
pub(super) const PATCH_FILTER_QUERY_JSON_KEY: &str = "Surge XT patch filter";

pub(crate) use self::cache::filter_items;
pub(in crate::tui) use self::cache::filter_patches;
use self::cache::{mark_cache_entry_recent, resolve_cached_samples, try_insert_cache};
use self::render_queue::{TuiRenderCompletion, TuiRenderJobStatus, TuiRenderQueue};
pub(in crate::tui) use self::session::PatchLoadState;
use crate::{config::Config, patches::PatchSortOrder};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Mode {
    Normal,
    Insert,
    PatchSelect,
    NotepadHistory,
    NotepadHistoryGuide,
    PatchPhrase,
    Help,
}

/// handle_normal の戻り値
enum NormalAction {
    Continue,
    Quit,
    LaunchDaw,
}

struct ActiveRenderGuard {
    counter: Arc<AtomicUsize>,
}

impl ActiveRenderGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self { counter }
    }
}

impl Drop for ActiveRenderGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index == max_chars {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

#[derive(Clone, PartialEq)]
pub(super) enum PlayState {
    Idle,
    Running(String), // レンダリング中
    Playing(String), // 演奏中
    Done(String),
    Err(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TuiRenderStatus {
    pub(super) active: usize,
    pub(super) workers: usize,
    pub(super) pending: usize,
    pub(super) pending_playback: usize,
}

pub struct TuiApp<'a> {
    pub(super) mode: Mode,
    pub(super) help_origin: Mode,
    pub(super) lines: Vec<String>,
    pub(super) cursor: usize,
    pub(super) list_state: ListState,
    pub(super) textarea: TextArea<'a>,
    cfg: Arc<Config>,
    entry_ptr: usize, // *const PluginEntry as usize。render_server backend では 0。
    pub(super) play_state: Arc<Mutex<PlayState>>,
    playback_session: Arc<AtomicU64>,
    pub(super) active_offline_render_count: Arc<AtomicUsize>,
    render_queue: TuiRenderQueue,
    active_sink: Arc<Mutex<Option<Arc<rodio::Sink>>>>,
    /// MML文字列 → レンダリング済みサンプルのキャッシュ
    pub(super) audio_cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    audio_cache_order: Arc<Mutex<VecDeque<String>>>,
    // 音色選択モード用
    /// バックグラウンドスレッドが収集したパッチリストの状態
    patch_load_state: Arc<Mutex<PatchLoadState>>,
    pub(super) random_patch_decks: crate::random::RandomIndexDecks,
    /// ソート切替に応じて並びが変わる (表示名, 小文字化済み) ペアのリスト
    pub(super) patch_all: Vec<(String, String)>,
    pub(super) patch_all_source_order: Vec<(String, String)>,
    pub(super) patch_query: String, // 検索クエリ
    pub(super) patch_query_textarea: TextArea<'a>,
    pub(super) patch_filtered: Vec<String>, // フィルタ結果（表示名のみ）
    pub(super) patch_cursor: usize,         // フィルタ結果内のカーソル位置
    pub(super) patch_list_state: ListState, // 音色選択リスト描画用
    pub(super) patch_favorite_items: Vec<String>,
    pub(super) patch_favorites_cursor: usize,
    pub(super) patch_favorites_state: ListState,
    pub(super) patch_select_focus: PatchSelectPane,
    pub(super) patch_select_filter_active: bool,
    pub(super) patch_select_sort_order: PatchSortOrder,
    pub(super) normal_page_size: usize,
    pub(super) patch_select_page_size: usize,
    pub(super) notepad_history_page_size: usize,
    pub(super) patch_phrase_page_size: usize,
    pub(super) patch_phrase_store: crate::history::PatchPhraseStore,
    pub(super) notepad_history_cursor: usize,
    pub(super) notepad_favorites_cursor: usize,
    pub(super) notepad_history_state: ListState,
    pub(super) notepad_favorites_state: ListState,
    pub(super) notepad_focus: PatchPhrasePane,
    pub(super) notepad_query: String,
    pub(super) notepad_query_textarea: TextArea<'a>,
    pub(super) notepad_filter_active: bool,
    pub(super) notepad_pending_delete: bool,
    pub(super) normal_pending_delete: bool,
    pub(super) yank_buffer: Option<String>,
    pub(super) patch_phrase_name: Option<String>,
    pub(super) patch_phrase_history_cursor: usize,
    pub(super) patch_phrase_favorites_cursor: usize,
    pub(super) patch_phrase_history_state: ListState,
    pub(super) patch_phrase_favorites_state: ListState,
    pub(super) patch_phrase_focus: PatchPhrasePane,
    pub(super) patch_phrase_query: String,
    pub(super) patch_phrase_query_textarea: TextArea<'a>,
    pub(super) patch_phrase_filter_active: bool,
    pub(super) patch_phrase_store_dirty: bool,
    /// 終了時 DAW モードだったかどうか（history.json に保存・復元する）
    pub(super) is_daw_mode: bool,
    startup_normal_cache_primed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PatchPhrasePane {
    History,
    Favorites,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PatchSelectPane {
    Patches,
    Favorites,
}

impl<'a> TuiApp<'a> {
    fn log_notepad_event(message: impl Into<String>) {
        #[cfg(not(test))]
        crate::logging::append_global_log_line(format!("notepad: {}", message.into()));
        #[cfg(test)]
        let _ = message.into();
    }

    fn kick_play(&self, mml: String) {
        let cfg = Arc::clone(&self.cfg);
        let state = Arc::clone(&self.play_state);
        let playback_session = Arc::clone(&self.playback_session);
        let active_sink = Arc::clone(&self.active_sink);
        let cache = Arc::clone(&self.audio_cache);
        let cache_order = Arc::clone(&self.audio_cache_order);
        let render_queue = self.render_queue.clone();
        let session = self.begin_playback_session();
        let mml_log = truncate_for_log(&mml, 120);

        let cache_guard = cache.lock().unwrap();
        let cached_samples = resolve_cached_samples(Some(&cache_guard), &mml);
        if cached_samples.is_some() {
            let mut cache_order = cache_order.lock().unwrap();
            mark_cache_entry_recent(&cache_guard, &mut cache_order, &mml);
        }
        drop(cache_guard);

        if let Some(samples) = cached_samples {
            // キャッシュヒット: レンダリングをスキップして即時再生
            let msg = format!("(cached) | {}", mml);
            Self::log_notepad_event(format!(
                "play request session={session} cache=hit mml=\"{mml_log}\""
            ));
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
            Self::log_notepad_event(format!(
                "play request session={session} cache=miss mml=\"{mml_log}\""
            ));
            self.set_play_state_if_current(session, PlayState::Running(mml.clone()));

            let response_rx = match render_queue.submit_playback(
                mml.clone(),
                session,
                Arc::clone(&playback_session),
            ) {
                Ok(response_rx) => response_rx,
                Err(error) => {
                    Self::log_notepad_event(format!(
                        "play render queue error session={session} err=\"{}\"",
                        truncate_for_log(&error.to_string(), 160)
                    ));
                    self.set_play_state_if_current(
                        session,
                        PlayState::Err(format!("エラー: {}", error)),
                    );
                    return;
                }
            };

            std::thread::spawn(move || {
                let response = match response_rx.recv() {
                    Ok(response) => response,
                    Err(_) => {
                        Self::log_notepad_event(format!(
                            "play render response dropped session={session}"
                        ));
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err("エラー: render queue response dropped".to_string()),
                        );
                        return;
                    }
                };

                match response.completion {
                    TuiRenderCompletion::SkippedStalePlayback => {}
                    TuiRenderCompletion::RenderError(error) => {
                        Self::log_notepad_event(format!(
                            "play render error session={session} err=\"{}\"",
                            truncate_for_log(&error, 160)
                        ));
                        Self::set_play_state_for_session(
                            &state,
                            &playback_session,
                            session,
                            PlayState::Err(format!("エラー: {}", error)),
                        );
                    }
                    TuiRenderCompletion::Rendered {
                        samples,
                        patch_name,
                    } => {
                        if !Self::playback_session_is_current(&playback_session, session) {
                            Self::log_notepad_event(format!(
                                "play render stale skip after-render session={session}"
                            ));
                            return;
                        }
                        {
                            let mut cache = cache.lock().unwrap();
                            let mut cache_order = cache_order.lock().unwrap();
                            try_insert_cache(
                                &mut cache,
                                &mut cache_order,
                                mml.clone(),
                                samples.clone(),
                                false,
                            );
                        }

                        let msg = format!("{} | {}", patch_name, mml);
                        // 演奏中に切り替え
                        Self::log_notepad_event(format!(
                            "play render ok session={session} patch=\"{}\"",
                            truncate_for_log(&patch_name, 120)
                        ));
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

    pub(super) fn active_parallel_render_count(&self) -> usize {
        self.active_offline_render_count.load(Ordering::Relaxed)
    }

    pub(super) fn render_status_snapshot(&self) -> TuiRenderStatus {
        let queue_stats = self.render_queue.stats();
        TuiRenderStatus {
            active: self.active_parallel_render_count(),
            workers: queue_stats.workers,
            pending: queue_stats.pending_jobs,
            pending_playback: queue_stats.pending_playback_jobs,
        }
    }

    pub(in crate::tui) fn render_job_status_for_mml(
        &self,
        mml: &str,
    ) -> Option<TuiRenderJobStatus> {
        let mml = mml.trim();
        if mml.is_empty() {
            return None;
        }
        self.render_queue.job_status(mml)
    }

    fn draw(&mut self, f: &mut Frame) {
        ui::draw(self, f);
    }
}

#[cfg(test)]
#[path = "../tests/tui_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "../tests/tui/mod.rs"]
mod tests;
