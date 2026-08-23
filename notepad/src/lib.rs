//! notepad 画面（MML を1行1フレーズで編集・再生するメイン画面）。
//!
//! 状態・キー入力・描画・再生・オフラインレンダリングのキャッシュはこの crate に
//! 閉じており、共有ランタイム（app 側の `TuiApp`）とは [`NotepadScreen`] を介して
//! やり取りする。app 側との接続は app crate の `tui::notepad_glue` にある。
//!
//! 画面そのものの切替（notepad / DAW / keyboard / loop browser）は app 側の
//! `active_screen` が持ち、この crate の [`Mode`] は notepad 内部のサブモードだけを表す。
//!
//! グローバルログへの書き込みは app 側の sink 注入（[`set_log_sink`]）で有効になる。
//! 未注入だとログが黙って消えるため、注入は app 起動時に必ず行うこと。

mod audio_cache;
mod cache;
mod disk_cache;
mod input;
mod logging;
mod notepad_editor;
mod notepad_history;
mod patch_phrase;
mod patch_select;
mod playback;
mod playback_runtime;
mod prefetch;
mod render_queue;
pub mod ui;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cmrt_runtime::Config;
use cmrt_tui_core::patch_load::PatchLoadState;
use cmrt_tui_core::playback_session::PlaybackSession;
use cmrt_tui_core::sound_check_guide::SoundCheckGuide;
pub use cmrt_tui_core::PlayState;

use audio_cache::NotepadAudioCache;
pub use logging::set_log_sink;
use notepad_editor::NotepadEditorState;
use notepad_history::NotepadHistoryState;
use patch_phrase::PatchPhraseState;
use patch_select::PatchSelectState;
use playback_runtime::TuiPlaybackRuntime;
pub use render_queue::TuiRenderJobStatus;
use render_queue::TuiRenderQueue;

/// audio_cache の最大エントリ数。超過時は古いエントリから1件ずつ退避する。
const AUDIO_CACHE_MAX_ENTRIES: usize = 64;
pub(crate) const PATCH_JSON_KEY: &str = "Surge XT patch";
pub(crate) const PATCH_FILTER_QUERY_JSON_KEY: &str = "Surge XT patch filter";

pub const NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE: &str = "j,kキーを押して音が鳴ることを確認してください";

// patch 表示パス / 文字列リストのフィルタは共有処理なので `cmrt-tui-core` にある。
// crate 内では短いパスで参照する。
pub(crate) use cmrt_tui_core::patches::{filter_items, filter_patches_by_display_path};

#[cfg(test)]
use cache::{mark_cache_entry_recent, resolve_cached_samples, try_insert_cache};

/// notepad 画面の内部モード。
///
/// どの主要画面を表示しているかは app 側の `PrimaryScreen` が持つ。
/// ここには notepad 内のサブモードだけを置く。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    PatchSelect,
    NotepadHistory,
    NotepadHistoryGuide,
    PatchPhrase,
    Help,
}

/// notepad のキー入力を処理した結果、共有ランタイム側に依頼したい操作。
pub enum NormalAction {
    Continue,
    Quit,
    LaunchDaw,
    LaunchKeyboard,
    LaunchLoopBrowser,
    EditConfig,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatchPhrasePane {
    History,
    Favorites,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatchSelectPane {
    Patches,
    Favorites,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TuiRenderStatus {
    pub(crate) active: usize,
    pub(crate) workers: usize,
    pub(crate) pending: usize,
    pub(crate) pending_playback: usize,
}

/// 並列レンダリング中の本数を数えるガード。生存期間中だけカウントを +1 する。
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

pub(crate) fn truncate_for_log(value: &str, max_chars: usize) -> String {
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

pub struct NotepadScreen<'a> {
    pub mode: Mode,
    pub help_origin: Mode,
    pub(crate) editor: NotepadEditorState<'a>,
    pub(crate) playback: TuiPlaybackRuntime,
    pub(crate) audio: NotepadAudioCache,
    pub(crate) sound_check_guide: SoundCheckGuide,
    /// ソート切替に応じて並びが変わる (表示名, 小文字化済み) ペアのリスト。
    /// バックグラウンド読み込みの共有ハンドルで、keyboard 画面とも同じ実体を見る。
    pub patch_load_state: Arc<Mutex<PatchLoadState>>,
    pub(crate) random_patch_decks: cmrt_tui_core::random::RandomIndexDecks,
    pub(crate) patch_select: PatchSelectState<'a>,
    pub(crate) notepad_history: NotepadHistoryState<'a>,
    pub(crate) patch_phrase: PatchPhraseState<'a>,
    pub(crate) patch_phrase_store: cmrt_history::PatchPhraseStore,
    pub(crate) patch_phrase_store_dirty: bool,
    pub(crate) startup_normal_cache_primed: bool,
    pub(crate) cfg: Arc<Config>,
    /// 設定不足でカタログから外れたプラグインの案内。音色選択の枠下へ出す。
    ///
    /// 一覧に**出てこない**ものの話なので、`patch_load_state` をいくら見ても分からない。
    /// config は起動中に変わらないので、組み立てのときに 1 回だけ数える。
    pub(crate) catalog_notes: Vec<String>,
}

/// [`NotepadScreen::new`] の引数一式。
///
/// 編集バッファ・レンダリングキューといった notepad 内部の型は crate 内に閉じたいので、
/// app からは復元済みのプリミティブ（行・カーソル位置・永続化データ）だけを渡す。
pub struct NotepadScreenParts {
    /// 復元した編集行。空でないこと（`load_session_state` が保証する）。
    pub lines: Vec<String>,
    /// 復元したカーソル行（0始まり）。`lines` の範囲へ丸めてから渡すこと。
    pub cursor: usize,
    /// 画面横断で共有する再生セッション。
    pub playback_session: PlaybackSession,
    /// 音出し確認ガイドを最後に overlay 表示したローカル日付（YYYY-MM-DD）。
    pub sound_check_guide_overlay_date: Option<String>,
    /// パッチ一覧のバックグラウンド読み込み状態（keyboard 画面と共有）。
    pub patch_load_state: Arc<Mutex<PatchLoadState>>,
    pub patch_phrase_store: cmrt_history::PatchPhraseStore,
    pub cfg: Arc<Config>,
    /// カタログのプラグインごとのロード済み CLAP entry。
    /// render_server backend / テストでは空。
    pub plugin_entries: cmrt_offline_render::PluginEntries,
    /// 設定不足でカタログから外れたプラグインの案内
    /// （`cmrt_runtime::catalog_notice_lines`）。
    ///
    /// **画面側では数えない。** 数えると実マシンのインストール状況を読むことになり、
    /// 画面のテストがマシン依存になる（`docs/adr/0005-mixed-catalog-on-by-default.md`）。
    pub catalog_notes: Vec<String>,
}

impl NotepadScreen<'static> {
    pub fn new(parts: NotepadScreenParts) -> Self {
        let NotepadScreenParts {
            lines,
            cursor,
            playback_session,
            sound_check_guide_overlay_date,
            patch_load_state,
            patch_phrase_store,
            cfg,
            plugin_entries,
            catalog_notes,
        } = parts;
        let active_offline_render_count = Arc::new(AtomicUsize::new(0));
        let render_queue = TuiRenderQueue::new(
            Arc::clone(&cfg),
            plugin_entries,
            Arc::clone(&active_offline_render_count),
        );
        Self::from_parts(
            NotepadEditorState::restored(lines, cursor),
            TuiPlaybackRuntime::new(playback_session, render_queue, active_offline_render_count),
            SoundCheckGuide::new(sound_check_guide_overlay_date),
            patch_load_state,
            patch_phrase_store,
            cfg,
            catalog_notes,
        )
    }

    /// レンダリングワーカーを起動しないテスト用の構築。
    #[cfg(any(test, feature = "test-support"))]
    pub fn new_for_test(cfg: Config) -> Self {
        let render_queue = TuiRenderQueue::disabled_for_tests(
            cfg.offline_render_backend,
            cfg.effective_offline_render_workers(),
        );
        Self::from_parts(
            NotepadEditorState::restored(vec![String::new()], 0),
            TuiPlaybackRuntime::new(
                PlaybackSession::new(None),
                render_queue,
                Arc::new(AtomicUsize::new(0)),
            ),
            SoundCheckGuide::new(None),
            Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new()))),
            cmrt_history::PatchPhraseStore::default(),
            Arc::new(cfg),
            Vec::new(),
        )
    }
}

impl<'a> NotepadScreen<'a> {
    fn from_parts(
        editor: NotepadEditorState<'a>,
        playback: TuiPlaybackRuntime,
        sound_check_guide: SoundCheckGuide,
        patch_load_state: Arc<Mutex<PatchLoadState>>,
        patch_phrase_store: cmrt_history::PatchPhraseStore,
        cfg: Arc<Config>,
        catalog_notes: Vec<String>,
    ) -> Self {
        Self {
            mode: Mode::Normal,
            help_origin: Mode::Normal,
            editor,
            playback,
            audio: NotepadAudioCache::new(),
            sound_check_guide,
            patch_load_state,
            random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
            patch_select: PatchSelectState::new(),
            notepad_history: NotepadHistoryState::new(),
            patch_phrase: PatchPhraseState::new(),
            patch_phrase_store,
            patch_phrase_store_dirty: false,
            startup_normal_cache_primed: false,
            catalog_notes,
            cfg,
        }
    }

    pub(crate) fn log_notepad_event(message: impl Into<String>) {
        logging::log_notepad_event(message);
    }

    pub fn begin_playback_session(&self) -> u64 {
        self.playback.session.begin()
    }

    /// 設定不足でカタログから外れたプラグインの案内。無ければ空。
    ///
    /// 画面が自分で数えたものを app 側が引く。**数えるのは 1 か所**にしておかないと、
    /// 画面ごとに違う案内が出る。文言は
    /// `cmrt_runtime::SkippedCatalogPlugin::notice_line` が単一ソース。
    pub fn catalog_notes(&self) -> &[String] {
        &self.catalog_notes
    }

    /// notepad のフレーズ履歴（履歴, お気に入り）。
    ///
    /// MML オーバーレイが読むために公開している。まだディスクへ書いていない分も
    /// 含めたいので、保存ファイルからではなくここから取ること。
    pub fn phrase_history(&self) -> (&[String], &[String]) {
        (
            &self.patch_phrase_store.notepad.history,
            &self.patch_phrase_store.notepad.favorites,
        )
    }

    pub fn set_play_state_if_current(&self, session: u64, next_state: PlayState) {
        self.playback
            .session
            .set_play_state_if_current(session, next_state);
    }

    pub(crate) fn active_parallel_render_count(&self) -> usize {
        self.playback
            .active_offline_render_count
            .load(Ordering::Relaxed)
    }

    pub(crate) fn render_status_snapshot(&self) -> TuiRenderStatus {
        let queue_stats = self.playback.render_queue.stats();
        TuiRenderStatus {
            active: self.active_parallel_render_count(),
            workers: queue_stats.workers,
            pending: queue_stats.pending_jobs,
            pending_playback: queue_stats.pending_playback_jobs,
        }
    }

    pub(crate) fn render_job_status_for_mml(&self, mml: &str) -> Option<TuiRenderJobStatus> {
        let mml = mml.trim();
        if mml.is_empty() {
            return None;
        }
        self.playback.render_queue.job_status(mml)
    }

    /// `audio_cache` の内容をディスクキャッシュへ書き出す。次回起動時の
    /// オフラインレンダリング待ちを省略できるようにするための処理で、
    /// `save_history_state()` と対になる終了処理として各終了パスで呼ぶ。
    pub fn flush_disk_cache(&self) {
        // notepad画面の実際のバッファ行に対応するエントリだけを永続ディスクキャッシュへ
        // 書き出す。patch select 等のプレビュー試聴で生成された、バッファに存在しない
        // 組み合わせ（パッチ名を変えただけの仮MMLなど）はここで除外し、上限
        // NOTEPAD_DISK_CACHE_MAX_FILES 件の永続キャッシュ枠を試聴で消費させない。
        let cache = self.audio.cache.lock().unwrap();
        let line_keys: HashSet<&str> = self.editor.lines.iter().map(|line| line.trim()).collect();
        let notepad_only: HashMap<String, Vec<f32>> = cache
            .iter()
            .filter(|(mml, _)| line_keys.contains(mml.as_str()))
            .map(|(mml, samples)| (mml.clone(), samples.clone()))
            .collect();
        drop(cache);
        disk_cache::flush_audio_cache_to_disk(&notepad_only, self.cfg.sample_rate as u32);
        *self.audio.known_disk_hashes.lock().unwrap() =
            disk_cache::scan_valid_cache_hashes(self.cfg.sample_rate as u32);
    }

    /// 画面横断で共有している再生セッション。
    pub fn playback_session(&self) -> &PlaybackSession {
        &self.playback.session
    }

    /// 編集行を差し替える（テスト用）。
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_session_lines_for_test(&mut self, lines: Vec<String>) {
        self.editor.lines = lines;
    }

    /// カーソル行を差し替える（テスト用）。リスト選択も追従させる。
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_session_cursor_for_test(&mut self, cursor: usize) {
        self.editor.cursor = cursor;
        self.editor.list_state.select(Some(cursor));
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn test_set_active_parallel_render_count(&self, count: usize) {
        self.playback
            .active_offline_render_count
            .store(count, Ordering::Relaxed);
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn test_set_render_job_status(
        &self,
        mml: impl Into<String>,
        status: Option<TuiRenderJobStatus>,
    ) {
        self.playback.render_queue.set_test_job_status(mml, status);
    }

    /// 起動時キャッシュの温めを済ませたか。
    pub fn startup_cache_primed(&self) -> bool {
        self.startup_normal_cache_primed
    }

    /// セッション保存用のカーソル行（0始まり）。
    pub fn session_cursor(&self) -> usize {
        self.editor.cursor
    }

    /// セッション保存用の編集行。
    pub fn session_lines(&self) -> &[String] {
        &self.editor.lines
    }

    /// 音出し確認ガイド（表示判定と最終 overlay 日付の保持）。
    pub fn sound_check_guide_mut(&mut self) -> &mut SoundCheckGuide {
        &mut self.sound_check_guide
    }

    pub fn sound_check_guide(&self) -> &SoundCheckGuide {
        &self.sound_check_guide
    }

    pub fn reset_sound_check_guide(&mut self) {
        self.sound_check_guide.reset_for_screen();
    }

    pub fn uses_textarea_cursor(&self) -> bool {
        match self.mode {
            Mode::Insert => true,
            Mode::PatchSelect => self.patch_select.patch_select_filter_active,
            Mode::NotepadHistory => self.notepad_history.filter_active,
            Mode::PatchPhrase => self.patch_phrase.filter_active,
            Mode::Normal | Mode::NotepadHistoryGuide | Mode::Help => false,
        }
    }

    /// 起動後に notepad 画面を初めて表示したときの1回だけ、ディスクキャッシュを
    /// 読み込んで各行のレンダリング済みサンプルを温める。
    ///
    /// `autoplay` が真なら、温め終わったあとカーソル行を再生する。
    pub fn prime_startup_cache(&mut self, autoplay: bool) {
        *self.audio.known_disk_hashes.lock().unwrap() =
            disk_cache::scan_valid_cache_hashes(self.cfg.sample_rate as u32);
        self.hydrate_all_lines_from_disk_cache_at_startup();
        self.prime_normal_mode_startup_cache();
        if autoplay {
            if let Some(mml) = self
                .editor
                .lines
                .get(self.editor.cursor)
                .map(|line| line.trim().to_string())
                .filter(|mml| !mml.is_empty())
            {
                self.kick_play(mml);
            }
        }
        self.startup_normal_cache_primed = true;
    }
}

#[cfg(test)]
mod tests;
