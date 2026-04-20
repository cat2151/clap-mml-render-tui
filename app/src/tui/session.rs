use clack_host::prelude::PluginEntry;
use ratatui::widgets::ListState;

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};

use super::{Mode, PatchPhrasePane, PatchSelectPane, PlayState, TuiApp, TuiRenderQueue};
use crate::{config::Config, patches::PatchSortOrder};

/// バックグラウンドパッチ読み込みの状態
pub(in crate::tui) enum PatchLoadState {
    Loading,
    Ready(Vec<(String, String)>), // (表示名, 小文字化済み表示名)
    Err(String),
}

struct LoadedSessionState {
    cursor: usize,
    lines: Vec<String>,
    list_state: ListState,
    is_daw_mode: bool,
}

/// 復元したセッションのカーソルを現在の行数に収まる範囲へ丸める。
///
/// `lines_len` は 1 以上であることを前提とする。
pub(super) fn clamp_session_cursor(cursor: usize, lines_len: usize) -> usize {
    debug_assert!(lines_len > 0, "session lines must not be empty");
    cursor.min(lines_len.saturating_sub(1))
}

fn load_initial_session_state() -> LoadedSessionState {
    // `lines` は常に1行以上を保持する（不変条件）。
    // load_session_state() は lines が空でないことを保証している。
    let crate::history::SessionState {
        cursor,
        lines,
        is_daw_mode,
    } = crate::history::load_session_state();
    let initial_cursor = clamp_session_cursor(cursor, lines.len());
    let mut list_state = ListState::default();
    list_state.select(Some(initial_cursor));
    LoadedSessionState {
        cursor: initial_cursor,
        lines,
        list_state,
        is_daw_mode,
    }
}

/// パッチ一覧の非同期読み込みを開始し、共有状態ハンドルを返す。
fn spawn_patch_loader(cfg: &Config) -> Arc<Mutex<PatchLoadState>> {
    // パッチリストはバックグラウンドスレッドで収集する。
    // 起動時の同期スキャンによる遅延を避けるため。
    let patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));
    let state_bg = Arc::clone(&patch_load_state);
    let cfg = cfg.clone();
    std::thread::spawn(move || match crate::patches::collect_patch_pairs(&cfg) {
        Ok(pairs) => {
            *state_bg.lock().unwrap() = PatchLoadState::Ready(pairs);
        }
        Err(e) => {
            *state_bg.lock().unwrap() = PatchLoadState::Err(e.to_string());
        }
    });
    patch_load_state
}

impl<'a> TuiApp<'a> {
    pub fn new(cfg: &'a Config, entry: &'a PluginEntry) -> Self {
        crate::logging::install_native_probe_logger();
        let cfg_arc = Arc::new(cfg.clone());
        let LoadedSessionState {
            cursor,
            lines,
            list_state,
            is_daw_mode,
        } = load_initial_session_state();
        let entry_ptr = entry as *const PluginEntry as usize;
        let active_offline_render_count = Arc::new(AtomicUsize::new(0));
        let render_queue = TuiRenderQueue::new(
            Arc::clone(&cfg_arc),
            entry_ptr,
            Arc::clone(&active_offline_render_count),
        );

        Self {
            mode: Mode::Normal,
            help_origin: Mode::Normal,
            lines,
            cursor,
            list_state,
            textarea: crate::text_input::new_single_line_textarea(""),
            cfg: Arc::clone(&cfg_arc),
            entry_ptr,
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            playback_session: Arc::new(AtomicU64::new(0)),
            active_offline_render_count,
            render_queue,
            active_sink: Arc::new(Mutex::new(None)),
            audio_cache: Arc::new(Mutex::new(HashMap::new())),
            audio_cache_order: Arc::new(Mutex::new(VecDeque::new())),
            patch_load_state: spawn_patch_loader(cfg),
            random_patch_decks: crate::random::RandomIndexDecks::default(),
            patch_all: Vec::new(),
            patch_all_source_order: Vec::new(),
            patch_query: String::new(),
            patch_query_textarea: crate::text_input::new_single_line_textarea(""),
            patch_filtered: Vec::new(),
            patch_cursor: 0,
            patch_list_state: ListState::default(),
            patch_favorite_items: Vec::new(),
            patch_favorites_cursor: 0,
            patch_favorites_state: ListState::default(),
            patch_select_focus: PatchSelectPane::Patches,
            patch_select_filter_active: false,
            patch_select_sort_order: PatchSortOrder::Path,
            normal_page_size: 1,
            patch_select_page_size: 1,
            notepad_history_page_size: 1,
            patch_phrase_page_size: 1,
            patch_phrase_store: crate::history::load_patch_phrase_store(),
            notepad_history_cursor: 0,
            notepad_favorites_cursor: 0,
            notepad_history_state: ListState::default(),
            notepad_favorites_state: ListState::default(),
            notepad_focus: PatchPhrasePane::History,
            notepad_query: String::new(),
            notepad_query_textarea: crate::text_input::new_single_line_textarea(""),
            notepad_filter_active: false,
            notepad_pending_delete: false,
            normal_pending_delete: false,
            yank_buffer: None,
            patch_phrase_name: None,
            patch_phrase_history_cursor: 0,
            patch_phrase_favorites_cursor: 0,
            patch_phrase_history_state: ListState::default(),
            patch_phrase_favorites_state: ListState::default(),
            patch_phrase_focus: PatchPhrasePane::History,
            patch_phrase_query: String::new(),
            patch_phrase_query_textarea: crate::text_input::new_single_line_textarea(""),
            patch_phrase_filter_active: false,
            patch_phrase_store_dirty: false,
            is_daw_mode,
            startup_normal_cache_primed: false,
        }
    }

    pub(super) fn save_history_state(&self) {
        let _ = crate::history::save_session_state(&crate::history::SessionState {
            cursor: self.cursor,
            lines: self.lines.clone(),
            is_daw_mode: self.is_daw_mode,
        });
    }
}
