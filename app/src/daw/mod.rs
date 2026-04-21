//! DAW 風モード
//!
//! 初回起動時は 9 tracks × (0..=8 measures) の matrix で開始する
//!   measure 0 = 音色 (timbre) / track ごとの共通ヘッダ
//!   track   0 = 拍子JSON + テンポ (例: `{"beat": "4/4"}t120`) → render 時に全小節の先頭にくっつける
//!
//! user は track 数・measure 数に対して実質無制限を求めている。
//! そのためアプリ側で 64 のような小さな固定上限を設けず、言語・OS・ライブラリが許す範囲で扱うこと。
//! 保存済みセッションが初期サイズより大きい場合は、そのサイズをそのまま受け入れる。
//!
//! キー操作 (NORMAL):
//!   Shift+H: history overlay を開く
//!   h / ←  : 小節 (列) を左へ移動
//!   l / →  : 小節 (列) を右へ移動
//!   j/k    : track (行) 移動
//!   M      : 中央 track へ移動
//!   L      : 末尾 track へ移動
//!   i      : INSERT モード（現在セルを編集）
//!   m      : mixer overlay を開く
//!   dd     : 現在セルを yank して空にする
//!   p      : yank 内容で現在セルを上書き
//!   u      : 直前の paste を 1 回だけ取り消す
//!   Enter / Space       : 非play時、現在 track の現在 meas を一発再生
//!   Shift+Enter         : 非play時、現在 meas の全 track を一発再生
//!   Shift+P             : 演奏 / 停止 toggle
//!   Shift+Space         : 非play時、現在 meas から演奏開始して継続
//!   s      : 現在 track の solo toggle
//!   r      : measure 0 にランダム音色を設定
//!   K / ?  : ヘルプ表示
//!   q      : アプリ終了
//!   n      : notepad へ切替
//!   ESC    : 反応なし
//!
//! キー操作 (MIXER):
//!   h/l    : track 移動
//!   j/k    : volume -/+3dB
//!   ESC    : overlay を閉じる → NORMAL
//!
//! キー操作 (HISTORY):
//!   n        : global history へ切り替え
//!   p        : current / selected patch history へ切り替え
//!   t        : patch select overlay へ切り替え
//!   h/l・←/→ : History/Favorites ペイン切り替え
//!   j/k      : 行移動
//!   Enter    : 選択内容を現在 track/meas に適用
//!   ESC      : overlay を閉じる → NORMAL
//!
//! キー操作 (PATCH SELECT):
//!   n        : global history へ切り替え
//!   p        : current / selected patch history へ切り替え
//!   t        : 現在選択 patch で開き直す
//!   /        : 絞り込み条件入力モード開始
//!   h/l・←/→ : (通常) Patches/Favorites ペイン切り替えして preview / (検索入力中) 無効
//!   j/k      : (通常) 行移動して preview / (検索入力中) 文字入力
//!   Space    : (通常) preview / (検索入力中) AND 条件
//!   Enter    : (通常) 選択 patch で現在 track の init meas を上書きして overlay を閉じる / (検索入力中) 絞り込みを確定（overlay 継続）
//!   ESC      : (通常) overlay を閉じる / (検索入力中) 絞り込み入力を中断
//!
//! キー操作 (INSERT):
//!   ESC   : 確定 → NORMAL
//!   Enter : 確定 → 次の小節へ移動 → INSERT 継続
//!   Ctrl+C / Ctrl+X / Ctrl+V : コピー / カット / ペースト
//!   (補足) MML 内で `;` を使うと、1 つの meas 内で複数フレーズを並べられる（再生時は各フレーズに音色/track0 を適用）
//!
//! キー操作 (HELP):
//!   ESC   : キャンセル → NORMAL

mod batch_logging;
mod cache;
mod http_server;
mod init;
mod input;
mod mixer;
mod mml;
mod playback;
mod playback_util;
mod preview;
mod render_queue;
mod runtime;
mod save;
mod timing;
mod types;
mod ui;
mod wav_io;

use cmrt_core::{ensure_daw_dir, write_wav};
use ratatui::Frame;
use tui_textarea::TextArea;

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use crate::config::Config;

// ─── 再エクスポート ───────────────────────────────────────────

use batch_logging::{TrackRerenderBatch, TrackRerenderBatchCompletionContext};
use render_queue::RenderQueue;
pub use types::DawExitReason;
pub(super) use types::{
    AbRepeatState, CacheState, CellCache, DawHistoryPane, DawMode, DawNormalAction,
    DawPatchSelectPane, DawPlayState, PlayPosition,
};

// ─── 定数 ─────────────────────────────────────────────────────

/// 初回起動時の track 数。track 0 = Tempo、track 1..=8 = 演奏 track。
pub const TRACKS: usize = 9;
/// 初回起動時の小節数。measure 0 = 音色列。measure 1..=MEASURES = 通常小節。
pub const MEASURES: usize = 8;
/// track 0 はグローバルヘッダ（テンポ等）専用。演奏 track は 1 から始まる。
pub(super) const FIRST_PLAYABLE_TRACK: usize = 1;
pub(super) const MIXER_MIN_DB: i32 = -36;
pub(super) const MIXER_MAX_DB: i32 = 6;
/// track 0 / measure 0 のデフォルト内容（拍子指定 JSON + テンポ設定）。
/// セーブファイルが存在しない初回起動時に使用される。
pub(super) const DEFAULT_TRACK0_MML: &str = r#"{"beat": "4/4"}t120"#;

/// インメモリキャッシュに保持するサンプル数の上限（ステレオ、インターリーブ）。
///
/// 2_000_000 サンプル / 2 ch = 1_000_000 samples per ch / 44100 Hz ≈ 22.7 秒 / 小節。
/// 4/4 拍子では BPM ≈ 4 * 60 / 22.7 ≈ 10.6 以上の小節がキャッシュ対象となる。
/// これを超えるサンプル数のセル（極端に低い BPM など）はキャッシュに保持せず、
/// 再生時にフォールバックレンダリングする。
/// ≈ 2_000_000 × 4 bytes ≈ 8 MB / cell。
pub(super) const MAX_CACHED_SAMPLES: usize = 2_000_000;
const OVERLAY_PREVIEW_CACHE_MAX_ENTRIES: usize = 64;

#[derive(Clone)]
pub(super) struct CacheJob {
    track: usize,
    measure: usize,
    measure_samples: usize,
    generation: u64,
    rendered_mml_hash: u64,
    mml: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NormalPasteUndo {
    track: usize,
    measure: usize,
    previous: String,
    pasted: String,
}

// ─── DawApp ───────────────────────────────────────────────────

pub struct DawApp {
    /// data[track][measure]: track 0..tracks, measure 0..=measures
    pub(super) data: Vec<Vec<String>>,

    pub(super) cursor_track: usize,   // 0..tracks-1
    pub(super) cursor_measure: usize, // 0..=measures  (0 = 音色列)

    pub(super) mode: DawMode,
    pub(super) help_origin: DawMode,
    pub(super) textarea: TextArea<'static>,

    cfg: Arc<Config>,
    entry_ptr: usize, // *const PluginEntry as usize。render_server backend では 0。

    /// 現在のトラック数（track 0 = ヘッダ/テンポ、track 1.. = 演奏トラック）
    pub(super) tracks: usize,
    /// 現在の小節数（measure 0 = 音色列、measure 1.. = 通常小節）
    pub(super) measures: usize,

    /// セルごとのキャッシュ [track][measure]
    pub(super) cache: Arc<Mutex<Vec<Vec<CellCache>>>>,

    /// キャッシュワーカースレッドへのジョブチャネル
    /// 設定数ワーカーで処理し、prepare 段階の排他は core-lib 側で行う
    cache_tx: std::sync::mpsc::Sender<CacheJob>,
    cache_render_workers: usize,
    render_queue: RenderQueue,

    pub(super) play_state: Arc<Mutex<DawPlayState>>,

    /// プレビュー開始と停止の遷移を直列化するロック。
    /// `stop_play()` と `start_preview()` の間で状態確認と音声キュー投入が
    /// 交錯しないようにする。
    play_transition_lock: Arc<Mutex<()>>,
    /// 現在の preview セッション ID。
    /// restart 時に古い preview スレッドが新しい状態を上書きしないようにする。
    preview_session: Arc<AtomicU64>,
    /// 現在アクティブな preview sink。
    /// preview restart 時に既存音声を即時停止するために保持する。
    preview_sink: Arc<Mutex<Option<Arc<rodio::Sink>>>>,

    /// 再生中の小節・ビート位置（UI 描画に使用）
    pub(super) play_position: Arc<Mutex<Option<PlayPosition>>>,
    pub(super) ab_repeat: Arc<Mutex<AbRepeatState>>,
    overlay_preview_cache: Arc<Mutex<HashMap<u64, Arc<Vec<f32>>>>>,

    /// 再生スレッドと共有する各小節の MML ベクター（measures 要素, index i → meas i+1）。
    /// セル編集・ランダム音色変更のたびに更新されることで、
    /// play 中でも次ループ冒頭から新しい MML が反映される（hot reload）。
    play_measure_mmls: Arc<Mutex<Vec<String>>>,
    /// 再生スレッドと共有する各小節・各 track の MML。
    /// index i → meas i+1, inner index t → track t.
    play_measure_track_mmls: Arc<Mutex<Vec<Vec<String>>>>,

    /// 再生スレッドと共有する 1 小節のステレオサンプル数。
    /// セル編集・ランダム音色変更のたびに `play_measure_mmls` と一緒に更新される。
    play_measure_samples: Arc<Mutex<usize>>,

    /// DAW モード下部に表示するデバッグログ。
    pub(super) log_lines: Arc<Mutex<VecDeque<String>>>,

    /// track ごとの再レンダリング進捗ログ管理。
    track_rerender_batches: Arc<Mutex<Vec<Option<TrackRerenderBatch>>>>,

    /// playable track ごとの solo 状態。いずれかが true の間だけ solo モード。
    pub(super) solo_tracks: Vec<bool>,
    /// playable track ごとの音量(dB)。
    pub(super) track_volumes_db: Vec<i32>,
    /// mixer overlay で選択中の track。
    pub(super) mixer_cursor_track: usize,
    /// 再生スレッドと共有する track ごとの gain。
    play_track_gains: Arc<Mutex<Vec<f32>>>,
    pub(super) yank_buffer: Option<String>,
    pub(super) normal_pending_delete: bool,
    pub(super) normal_paste_undo: Option<NormalPasteUndo>,
    pub(super) patch_phrase_store: crate::history::PatchPhraseStore,
    pub(super) patch_phrase_store_dirty: bool,
    pub(super) history_overlay_patch_name: Option<String>,
    pub(super) history_overlay_query: String,
    pub(super) history_overlay_query_textarea: TextArea<'static>,
    pub(super) history_overlay_history_cursor: usize,
    pub(super) history_overlay_favorites_cursor: usize,
    pub(super) history_overlay_focus: DawHistoryPane,
    pub(super) history_overlay_filter_active: bool,
    pub(super) patch_all: Vec<(String, String)>,
    pub(super) patch_query: String,
    pub(super) patch_query_textarea: TextArea<'static>,
    pub(super) patch_query_before_input: String,
    pub(super) patch_filtered: Vec<String>,
    pub(super) patch_cursor: usize,
    pub(super) patch_favorite_items: Vec<String>,
    pub(super) patch_favorites_cursor: usize,
    pub(super) patch_select_focus: DawPatchSelectPane,
    pub(super) patch_select_filter_active: bool,
    pub(super) random_patch_decks: crate::random::RandomIndexDecks,
}

impl DawApp {
    pub fn new(cfg: Arc<Config>, entry_ptr: usize) -> Self {
        init::new(cfg, entry_ptr)
    }

    fn offline_render_available(&self) -> bool {
        self.entry_ptr != 0
            || self.cfg.offline_render_backend == crate::config::OfflineRenderBackend::RenderServer
    }

    pub(super) fn ab_repeat_state(&self) -> AbRepeatState {
        *self.ab_repeat.lock().unwrap()
    }

    // ─── ランダム音色 ─────────────────────────────────────────

    fn patch_query_terms(query: Option<&str>) -> Option<Vec<String>> {
        query
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .map(|query| {
                query
                    .split_whitespace()
                    .map(|term| term.to_lowercase())
                    .collect()
            })
    }

    fn patch_matches_query(lower_patch_name: &str, terms: &[String]) -> bool {
        terms
            .iter()
            .all(|term| lower_patch_name.contains(term.as_str()))
    }

    fn filter_patch_pairs_by_query(
        patches: Vec<(String, String)>,
        query: Option<&str>,
    ) -> Vec<(String, String)> {
        let Some(terms) = Self::patch_query_terms(query) else {
            return patches;
        };
        patches
            .into_iter()
            .filter(|(_, lower)| Self::patch_matches_query(lower, &terms))
            .collect()
    }

    fn filter_patch_names_by_query(all: &[(String, String)], query: &str) -> Vec<String> {
        let Some(terms) = Self::patch_query_terms(Some(query)) else {
            return all.iter().map(|(orig, _)| orig.clone()).collect();
        };
        all.iter()
            .filter(|(_, lower)| Self::patch_matches_query(lower, &terms))
            .map(|(orig, _)| orig.clone())
            .collect()
    }

    fn pick_random_patch_name(&mut self) -> Option<String> {
        self.pick_random_patch_name_with_query(None)
    }

    fn pick_random_patch_name_with_query(&mut self, query: Option<&str>) -> Option<String> {
        let patches = crate::patches::collect_patch_pairs(&self.cfg).ok()?;
        let candidates = Self::filter_patch_pairs_by_query(patches, query)
            .into_iter()
            .map(|(orig, _)| orig)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let idx = self
            .random_patch_decks
            .next_index(query, candidates.len())?;
        Some(candidates[idx].clone())
    }

    // ─── 描画 ─────────────────────────────────────────────────

    fn draw(&self, f: &mut Frame) {
        ui::draw(self, f);
    }

    fn append_log_line(&self, message: impl Into<String>) {
        crate::logging::append_log_line(&self.log_lines, message);
    }
}

pub(crate) fn ensure_http_server_for_mode_switch() {
    http_server::ensure_daw_http_server_thread();
}

#[cfg(test)]
pub(crate) fn request_http_mode_switch() {
    http_server::request_daw_mode_switch();
}

pub(crate) fn take_http_mode_switch_request() -> bool {
    http_server::take_daw_mode_switch_request()
}

#[cfg(test)]
#[path = "../tests/daw/mod.rs"]
mod tests;
