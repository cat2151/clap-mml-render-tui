//! vim 風 TUI
//!
//! モード:
//!   NORMAL : j/k で行移動、PageUp/PageDown で1画面移動、Home/M/L で先頭/中央/末尾行へ移動、i/o で INSERT、dd / Delete で現在行をヤンク削除、g で generate を現在行の上へ挿入して再生、r で現在行の先頭にランダム音色を挿入/置換、t で音色選択、Shift+H で patch history（patch name が無い場合は notepad history 案内）、f で patch history、w で DAW、Enter/Space で再生、q で終了
//!   INSERT : tui-textarea で編集
//!            ESC   → 確定 → NORMAL（再生開始）
//!            Enter → 確定 → 次行に新規行挿入 → INSERT 継続
//!            Ctrl+C / Ctrl+X / Ctrl+V → コピー / カット / ペースト
//!   PATCHSELECT : 音色を選択
//!            / の後に文字入力: patch name フィルタ（space=AND条件）
//!            n/p/t: notepad history / patch history / 音色選択
//!            j/k・↑↓・PageUp/PageDown:リスト移動（移動ごとにpreview再生）
//!            h/l・←/→:左右ペイン移動（移動ごとにpreview再生）
//!            f:現在音色とMMLをFavorites追加
//!            Enter:現在行の先頭にJSONで挿入（上書き）  ESC:キャンセル
//!   HELP : K / ? で表示、ESC でキャンセル

mod cache;
mod input;
mod notepad_history;
mod patch_phrase;
mod playback_session;
mod runtime;
pub(crate) mod session;
mod ui;

use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render, CoreConfig};
use ratatui::{widgets::ListState, Frame};
use tui_textarea::TextArea;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

/// audio_cache の最大エントリ数。超過時はキャッシュ全体をクリアしてから挿入する。
const AUDIO_CACHE_MAX_ENTRIES: usize = 64;
pub(super) const PATCH_JSON_KEY: &str = "Surge XT patch";

pub(crate) use self::cache::{filter_items, filter_patches};
use self::cache::{resolve_cached_samples, try_insert_cache};
pub(crate) use self::session::PatchLoadState;
use crate::config::Config;

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
    pub(super) help_origin: Mode,
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
    pub(super) patch_favorite_items: Vec<String>,
    pub(super) patch_favorites_cursor: usize,
    pub(super) patch_favorites_state: ListState,
    pub(super) patch_select_focus: PatchSelectPane,
    pub(super) patch_select_filter_active: bool,
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
    pub(super) patch_phrase_filter_active: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PatchSelectPane {
    Patches,
    Favorites,
}

impl<'a> TuiApp<'a> {
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
}

#[cfg(test)]
#[path = "../tests/tui_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "../tests/tui/mod.rs"]
mod tests;
