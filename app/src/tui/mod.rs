//! vim 風 TUI
//!
//! モード:
//!   NORMAL : j/k で行移動、PageUp/PageDown で1画面移動、Home/M で先頭/中央行へ移動、i/o で INSERT、dd / Delete で現在行をヤンク削除、g で generate を現在行の上へ挿入して再生、r で現在行の先頭にランダム音色を挿入/置換、t で音色選択、Shift+H で patch history（patch name が無い場合は notepad history 案内）、f で patch history、e で config.toml 編集、b で WAV loop browser、w で DAW、Enter/Space で再生、q で終了
//!   INSERT : tui-textarea で編集
//!            ESC   → 確定 → NORMAL（再生開始）
//!            Enter → 確定 → 次行に新規行挿入 → INSERT 継続
//!            Ctrl+C / Ctrl+X / Ctrl+V → コピー / カット / ペースト
//!   PATCHSELECT : 音色を選択
//!            / の後に文字入力: 現在paneの patch name フィルタ（space=AND条件）
//!            n/p/t: notepad history / patch history / 音色選択
//!            j/k・↑↓・PageUp/PageDown:リスト移動（移動ごとにpreview再生）
//!            h/l・←/→:左右ペイン移動（移動ごとにpreview再生）
//!            Ctrl+S:sort順切替
//!            f:現在音色とMMLをFavorites追加
//!            Enter:現在行の先頭にJSONで挿入（上書き）  ESC:キャンセル
//!   HELP : K / ? で表示、ESC でキャンセル

mod cache;
mod disk_cache;
mod input;
mod keyboard;
mod loop_browser;
mod notepad_history;
mod patch_phrase;
mod patch_select;
mod playback;
mod playback_session;
mod prefetch;
mod render_queue;
mod runtime;
mod session;
mod ui;

use ratatui::{widgets::ListState, Frame};
use tui_textarea::TextArea;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// audio_cache の最大エントリ数。超過時は古いエントリから1件ずつ退避する。
const AUDIO_CACHE_MAX_ENTRIES: usize = 64;
pub(super) const PATCH_JSON_KEY: &str = "Surge XT patch";
pub(super) const PATCH_FILTER_QUERY_JSON_KEY: &str = "Surge XT patch filter";

pub(crate) use self::cache::filter_items;
pub(in crate::tui) use self::cache::filter_patches;
use self::keyboard::{KeyboardMidiSender, KeyboardMmlInput, KeyboardNoteGuide, KeyboardState};
use self::notepad_history::NotepadHistoryState;
use self::patch_select::PatchSelectState;
use self::render_queue::{TuiRenderJobStatus, TuiRenderQueue};
pub(in crate::tui) use self::session::PatchLoadState;
use crate::{config::Config, realtime_play::RealtimePlayServerSupervisor};

pub(in crate::tui) const NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE: &str =
    "j,kキーを押して音が鳴ることを確認してください";

#[cfg(test)]
use self::cache::{mark_cache_entry_recent, resolve_cached_samples, try_insert_cache};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Mode {
    Normal,
    Insert,
    PatchSelect,
    NotepadHistory,
    NotepadHistoryGuide,
    PatchPhrase,
    Help,
    Keyboard,
    LoopBrowser,
}

/// handle_normal の戻り値
enum NormalAction {
    Continue,
    Quit,
    LaunchDaw,
    LaunchKeyboard,
    LaunchLoopBrowser,
    EditConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiExitReason {
    Quit,
    RestartApp,
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
    realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
    keyboard_midi_sender: Option<KeyboardMidiSender>,
    pub(super) keyboard_state: KeyboardState,
    pub(super) keyboard_mml_input: KeyboardMmlInput<'a>,
    pub(super) keyboard_note_guide: KeyboardNoteGuide,
    pub(super) notepad_sound_check_guide: crate::sound_check_guide::SoundCheckGuide,
    /// patch ごとの mono/poly 判定結果のキャッシュ。起動時に読み込み、
    /// 新しく probe した patch を検出したら書き戻す。
    pub(super) voicing_cache: crate::history::VoicingCache,
    pub(super) voicing_layers: crate::voicing_sources::VoicingLayers,
    pub(super) voicing_source_refresh: crate::voicing_sources::VoicingSourceRefresh,
    pub(super) persist_keyboard_on_exit: bool,
    pub(super) active_offline_render_count: Arc<AtomicUsize>,
    render_queue: TuiRenderQueue,
    active_sink: Arc<Mutex<Option<Arc<rodio::Sink>>>>,
    /// MML文字列 → レンダリング済みサンプルのキャッシュ
    pub(super) audio_cache: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    audio_cache_order: Arc<Mutex<VecDeque<String>>>,
    /// notepad ディスクキャッシュディレクトリに現在存在する、有効な wav の MML ハッシュ集合。
    /// `audio_cache` から追い出された行のディスクフォールバック判定と、一覧UIでの
    /// 即再生可能マーク表示に使う。起動時と `flush_notepad_disk_cache` 実行後に更新する。
    pub(super) known_disk_cache_hashes: Arc<Mutex<HashSet<u64>>>,
    // 音色選択モード用
    /// バックグラウンドスレッドが収集したパッチリストの状態
    patch_load_state: Arc<Mutex<PatchLoadState>>,
    pub(super) random_patch_decks: crate::random::RandomIndexDecks,
    /// ソート切替に応じて並びが変わる (表示名, 小文字化済み) ペアのリスト
    pub(in crate::tui) patch_select: PatchSelectState<'a>,
    pub(super) normal_page_size: usize,
    pub(in crate::tui) notepad_history: NotepadHistoryState<'a>,
    pub(super) patch_phrase_page_size: usize,
    pub(super) patch_phrase_store: crate::history::PatchPhraseStore,
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
    pub(super) loop_browser: loop_browser::LoopBrowser,
    loop_playback: Option<loop_browser::playback::LoopPlaybackController>,
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

    /// `audio_cache` の内容をディスクキャッシュへ書き出す。次回起動時の
    /// オフラインレンダリング待ちを省略できるようにするための処理で、
    /// `save_history_state()` と対になる終了処理として各終了パスで呼ぶ。
    pub(super) fn flush_notepad_disk_cache(&self) {
        // notepad画面の実際のバッファ行に対応するエントリだけを永続ディスクキャッシュへ
        // 書き出す。patch select 等のプレビュー試聴で生成された、バッファに存在しない
        // 組み合わせ（パッチ名を変えただけの仮MMLなど）はここで除外し、上限
        // NOTEPAD_DISK_CACHE_MAX_FILES 件の永続キャッシュ枠を試聴で消費させない。
        let cache = self.audio_cache.lock().unwrap();
        let line_keys: HashSet<&str> = self.lines.iter().map(|line| line.trim()).collect();
        let notepad_only: HashMap<String, Vec<f32>> = cache
            .iter()
            .filter(|(mml, _)| line_keys.contains(mml.as_str()))
            .map(|(mml, samples)| (mml.clone(), samples.clone()))
            .collect();
        drop(cache);
        disk_cache::flush_audio_cache_to_disk(&notepad_only, self.cfg.sample_rate as u32);
        *self.known_disk_cache_hashes.lock().unwrap() =
            disk_cache::scan_valid_cache_hashes(self.cfg.sample_rate as u32);
    }
}

#[cfg(test)]
#[path = "../tests/tui_helpers.rs"]
mod test_helpers;

#[cfg(test)]
#[path = "../tests/tui/mod.rs"]
mod tests;
