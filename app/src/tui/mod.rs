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

    fn sync_overlay_list_offset(
        state: &mut ListState,
        cursor: usize,
        item_count: usize,
        page_size: usize,
    ) {
        if item_count == 0 {
            *state.offset_mut() = 0;
            return;
        }

        let visible_count = page_size.max(1).min(item_count);
        let margin = visible_count.div_ceil(3);
        let max_offset = item_count.saturating_sub(visible_count);
        let current_offset = state.offset().min(max_offset);
        let top_threshold = current_offset.saturating_add(margin);
        let bottom_anchor = visible_count.saturating_sub(margin + 1);
        let desired_offset = if cursor < top_threshold {
            cursor.saturating_sub(margin)
        } else if cursor > current_offset.saturating_add(bottom_anchor) {
            cursor.saturating_sub(bottom_anchor)
        } else {
            current_offset
        };
        *state.offset_mut() = desired_offset.min(max_offset);
    }

    fn filtered_prefetch_targets(&self, mmls: Vec<String>) -> Vec<String> {
        let cache = self.audio_cache.lock().unwrap();
        let mut targets = Vec::new();
        for mml in mmls.into_iter().map(|mml| mml.trim().to_string()) {
            if mml.is_empty() || cache.contains_key(&mml) || targets.contains(&mml) {
                continue;
            }
            targets.push(mml);
        }
        targets
    }

    #[cfg(test)]
    fn insert_prefetch_targets_for_tests(&self, targets: Vec<String>) {
        let mut cache = self.audio_cache.lock().unwrap();
        let mut cache_order = self.audio_cache_order.lock().unwrap();
        for mml in targets {
            try_insert_cache(&mut cache, &mut cache_order, mml, Vec::new(), false);
        }
    }

    fn queue_prefetch_targets(
        cache: &Arc<Mutex<HashMap<String, Vec<f32>>>>,
        render_queue: &TuiRenderQueue,
        targets: Vec<String>,
    ) -> Vec<std::sync::mpsc::Receiver<self::render_queue::TuiRenderResponse>> {
        let prefetch_generation = render_queue.reserve_prefetch_generation();
        targets
            .into_iter()
            .filter_map(|mml| {
                if cache.lock().unwrap().contains_key(&mml) {
                    return None;
                }
                match render_queue.submit_prefetch(mml.clone(), prefetch_generation) {
                    Ok(response_rx) => Some(response_rx),
                    Err(error) => {
                        Self::log_notepad_event(format!(
                            "cache prefetch queue error err=\"{}\" mml=\"{}\"",
                            truncate_for_log(&error.to_string(), 160),
                            truncate_for_log(&mml, 80)
                        ));
                        None
                    }
                }
            })
            .collect()
    }

    fn consume_prefetch_response(
        cache: &Arc<Mutex<HashMap<String, Vec<f32>>>>,
        cache_order: &Arc<Mutex<VecDeque<String>>>,
        response_rx: std::sync::mpsc::Receiver<self::render_queue::TuiRenderResponse>,
    ) {
        let Ok(response) = response_rx.recv() else {
            Self::log_notepad_event("cache prefetch render response dropped");
            return;
        };
        match response.completion {
            TuiRenderCompletion::Rendered { samples, .. } => {
                let mut cache = cache.lock().unwrap();
                let mut cache_order = cache_order.lock().unwrap();
                try_insert_cache(&mut cache, &mut cache_order, response.mml, samples, false);
                Self::log_notepad_event("cache prefetch render ok");
            }
            TuiRenderCompletion::RenderError(error) => {
                Self::log_notepad_event(format!(
                    "cache prefetch render error mml=\"{}\" err=\"{}\"",
                    truncate_for_log(&response.mml, 80),
                    truncate_for_log(&error, 160)
                ));
            }
            TuiRenderCompletion::SkippedStalePlayback => {}
        }
    }

    fn render_queue_is_relaxed(
        render_queue: &TuiRenderQueue,
        active_offline_render_count: &AtomicUsize,
    ) -> bool {
        let stats = render_queue.stats();
        active_offline_render_count.load(Ordering::Relaxed) + stats.pending_jobs <= 1
    }

    fn prefetch_audio_cache_with_idle_fill(
        &self,
        immediate_mmls: Vec<String>,
        idle_mmls: Vec<String>,
    ) {
        let immediate_targets = self.filtered_prefetch_targets(immediate_mmls);
        let idle_targets = self.filtered_prefetch_targets(idle_mmls);
        if immediate_targets.is_empty() && idle_targets.is_empty() {
            return;
        }
        let target_count = immediate_targets.len() + idle_targets.len();
        Self::log_notepad_event(format!("cache prefetch request count={target_count}"));

        #[cfg(test)]
        if self.entry_ptr == 0 {
            self.insert_prefetch_targets_for_tests(immediate_targets);
            if self.render_queue.stats().pending_jobs == 0 {
                self.insert_prefetch_targets_for_tests(idle_targets);
            }
            return;
        }

        let cache = Arc::clone(&self.audio_cache);
        let cache_order = Arc::clone(&self.audio_cache_order);
        let render_queue = self.render_queue.clone();
        let active_offline_render_count = Arc::clone(&self.active_offline_render_count);
        let immediate_response_rxs =
            Self::queue_prefetch_targets(&cache, &render_queue, immediate_targets);

        if immediate_response_rxs.is_empty() && idle_targets.is_empty() {
            return;
        }

        std::thread::spawn(move || {
            let mut idle_targets = VecDeque::from(idle_targets);
            let mut response_rxs = VecDeque::from(immediate_response_rxs);

            if response_rxs.is_empty()
                && !idle_targets.is_empty()
                && Self::render_queue_is_relaxed(&render_queue, &active_offline_render_count)
            {
                if let Some(next_idle) = idle_targets.pop_front() {
                    response_rxs.extend(Self::queue_prefetch_targets(
                        &cache,
                        &render_queue,
                        vec![next_idle],
                    ));
                }
            }

            while let Some(response_rx) = response_rxs.pop_front() {
                Self::consume_prefetch_response(&cache, &cache_order, response_rx);
                if !idle_targets.is_empty()
                    && Self::render_queue_is_relaxed(&render_queue, &active_offline_render_count)
                {
                    if let Some(next_idle) = idle_targets.pop_front() {
                        response_rxs.extend(Self::queue_prefetch_targets(
                            &cache,
                            &render_queue,
                            vec![next_idle],
                        ));
                    }
                }
            }
        });
    }

    fn prefetch_navigation_audio_cache<F>(
        &self,
        current: usize,
        item_count: usize,
        page_size: usize,
        preferred_delta: Option<isize>,
        mml_for_index: F,
    ) where
        F: FnMut(usize) -> Option<String>,
    {
        let immediate_indices = match preferred_delta {
            Some(delta) => crate::ui_utils::predicted_navigation_indices_in_direction(
                current, item_count, delta, 2,
            ),
            None => crate::ui_utils::predicted_navigation_indices(current, item_count, page_size),
        };
        let idle_indices = preferred_delta
            .map(|_| crate::ui_utils::predicted_navigation_indices(current, item_count, page_size))
            .unwrap_or_default()
            .into_iter()
            .filter(|index| !immediate_indices.contains(index))
            .collect::<Vec<_>>();
        let mut mml_for_index = mml_for_index;
        let immediate_targets = immediate_indices
            .into_iter()
            .filter_map(&mut mml_for_index)
            .collect::<Vec<_>>();
        let idle_targets = idle_indices
            .into_iter()
            .filter_map(mml_for_index)
            .collect::<Vec<_>>();
        self.prefetch_audio_cache_with_idle_fill(immediate_targets, idle_targets);
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
