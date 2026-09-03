//! DAW 風モード
//!
//! 初回起動時は 10 tracks × (0..=8 measures) の matrix で開始する
//!   measure 0 = 音色 (timbre) / track ごとの共通ヘッダ
//!   track   0 = 拍子JSON + テンポ (例: `{"beat": "4/4"}t120`) → render 時に全小節の先頭にくっつける
//!   track   1 = chord 行（コード進行を書く専用行。音は鳴らさない。`tracks` モジュール参照）
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
//!   i      : MML 入力オーバーレイ（1 行モード）で現在セルを編集
//!            （init 列だけは従来のインライン INSERT に落ちる）
//!   Ctrl+P : 同上（`i` と同じ入口。init 列では開かない）
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
//!   f      : Persistent で project file overlay を開く（a: Save As / o: Open / d: Open Daily Archive）
//!   e      : config.toml を editor で開く
//!   K / ?  : ヘルプ表示
//!   q      : アプリ終了
//!   n      : notepad へ切替
//!   v      : keyboard へ切替
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
//!   /        : 現在paneの絞り込み条件入力モード開始
//!   h/l・←/→ : (通常) Patches/Favorites ペイン切り替えして preview / (検索入力中) 無効
//!   j/k      : (通常) 行移動して preview / (検索入力中) 文字入力
//!   Space    : (通常) preview / (検索入力中) AND 条件
//!   Enter    : (通常) 選択 patch で現在 track の init meas を上書きして overlay を閉じる / (検索入力中) 絞り込みを確定（overlay 継続）
//!   ESC      : (通常) overlay を閉じる / (検索入力中) 絞り込み入力を中断
//!
//! キー操作 (MML 入力オーバーレイ):
//!   Enter      : 確定 → 次の小節の入力欄を開く
//!   ESC        : 確定 → 閉じる → NORMAL
//!   Ctrl+T     : 音色選択（確定するとその track の init 列へ反映）
//!   Ctrl+O     : フレーズ履歴
//!   Ctrl+L     : 演奏設定 (repeat / CC1 / velocity)
//!   Ctrl+Space : 現在行を鳴らし直す
//!
//! キー操作 (INSERT: init 列専用):
//!   ESC   : 確定 → NORMAL
//!   Enter : 確定 → 次の小節へ移動 → INSERT 継続
//!   Ctrl+C / Ctrl+X / Ctrl+V : コピー / カット / ペースト
//!   (補足) MML 内で `;` を使うと、1 つの meas 内で複数フレーズを並べられる（再生時は各フレーズに音色/track0 を適用）
//!
//! キー操作 (HELP):
//!   ESC   : キャンセル → NORMAL
//!
//! キー操作 (PROJECT):
//!   a     : Save As path 入力
//!   o     : Open path 入力
//!   d     : managed Daily Archive を copy として Open
//!   Enter : path 入力を実行
//!   ESC   : path 入力を戻る / overlay を閉じる

mod auto_trim;
mod batch_logging;
mod cache;
mod daily;
mod editor;
mod grid_import;
mod grid_preview;
mod guide;
mod http_server;
mod init;
mod input;
mod live_instance;
mod messages;
mod mixer;
mod mml;
mod mml_overlay_glue;
mod overlays;
mod patch_catalog;
mod playback;
mod playback_runtime;
mod playback_util;
mod preview;
mod project;
mod render_queue;
mod runtime;
mod save;
mod timing;
mod tracks;
mod types;
mod ui;

pub use grid_import::{
    DawGridChordBinding, DawGridChordSource, DawGridChordVoicing, DawGridImportSong,
    DawGridImportTrack, DawGridLane, DawGridNoteStep,
};
pub use grid_preview::{DawGridPreviewPlayer, DawGridPreviewStatus};

use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};
use ratatui_textarea::TextArea;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

use cmrt_runtime::Config;

// ─── config.toml 編集フック（app ポリシー注入）──────────────────
//
// terminal を suspend して外部 editor を起動する処理は app 側のポリシーのため、
// crate は注入された関数を呼ぶだけにする（main.rs で `set_config_editor` を注入）。
// log sink 注入と同型の依存性逆転。

/// config.toml を editor で開くための注入フックの型。
pub type ConfigEditorFn =
    fn(&mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()>;

static CONFIG_EDITOR: OnceLock<ConfigEditorFn> = OnceLock::new();

/// app から config.toml 編集関数を注入する。
pub fn set_config_editor(editor: ConfigEditorFn) {
    let _ = CONFIG_EDITOR.set(editor);
}

/// 注入された config.toml 編集関数を呼ぶ。未注入時は no-op（`Ok(())`）。
pub(crate) fn edit_config_toml(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<()> {
    match CONFIG_EDITOR.get() {
        Some(editor) => editor(terminal),
        None => Ok(()),
    }
}

// ─── コード進行カタログ（app ポリシー注入）────────────────────

/// chord wizard（`G`）へ渡す、コード進行（degrees 文字列）の一覧を返す関数。
///
/// カタログの取得・ネットワーク・キャッシュは app 側の責務なので、DAW は
/// 「一覧を返す関数」だけを受け取る。`cmrt-chord` へは依存しない
/// （`parse_chord_progression` を DAW へ持ち込まないため。資料 3.4）。
///
/// **遅延評価**にしてあるのは、カタログのキャッシュがまだ無い初回に app 側の
/// 取得が最大 20 秒待つため。DAW 画面へ入るたびに待たされないよう、
/// `G` を押したときにだけ呼ぶ。
pub type ChordProgressionSource = Arc<dyn Fn() -> Vec<String> + Send + Sync>;

// ─── 再エクスポート ───────────────────────────────────────────

use batch_logging::{TrackRerenderBatch, TrackRerenderBatchCompletionContext};
use editor::DawEditorState;
use overlays::DawOverlays;
use playback_runtime::DawPlaybackRuntime;
use render_queue::RenderQueue;
pub(crate) use types::{
    AbRepeatState, CacheState, CellCache, DawHistoryPane, DawMode, DawNormalAction,
    DawPatchSelectPane, DawPlayState, DawProjectFileAction, PlayPosition,
};
pub use types::{DawExitReason, WorkspaceKind};

// ─── 定数 ─────────────────────────────────────────────────────

/// 初回起動時の track 数。track 0 = Tempo、track 1 = chord 行、track 2..=9 = 演奏 track。
pub const TRACKS: usize = 10;
/// 初回起動時の小節数。measure 0 = 音色列。measure 1..=MEASURES = 通常小節。
pub const MEASURES: usize = 8;
pub(crate) use cmrt_tui_core::mixer::{MIXER_MAX_DB, MIXER_MIN_DB};
pub(crate) use tracks::{CHORD_TRACK, FIRST_PLAYABLE_TRACK};
/// track 0 / measure 0 のデフォルト内容（拍子指定 JSON + テンポ設定）。
/// セーブファイルが存在しない初回起動時に使用される。
pub(crate) const DEFAULT_TRACK0_MML: &str = r#"{"beat": "4/4"}t120"#;

/// インメモリキャッシュに保持するサンプル数の上限（ステレオ、インターリーブ）。
///
/// 2_000_000 サンプル / 2 ch = 1_000_000 samples per ch / 44100 Hz ≈ 22.7 秒 / 小節。
/// 4/4 拍子では BPM ≈ 4 * 60 / 22.7 ≈ 10.6 以上の小節がキャッシュ対象となる。
/// これを超えるサンプル数のセル（極端に低い BPM など）はキャッシュに保持せず、
/// 再生時にフォールバックレンダリングする。
/// ≈ 2_000_000 × 4 bytes ≈ 8 MB / cell。
pub(crate) const MAX_CACHED_SAMPLES: usize = 2_000_000;
const OVERLAY_PREVIEW_CACHE_MAX_ENTRIES: usize = 64;
pub(crate) const DAW_SOUND_CHECK_GUIDE_MESSAGE: &str =
    "h,j,k,lキーを押して音が鳴ることを確認してください";

#[derive(Clone)]
pub(crate) struct CacheJob {
    track: usize,
    measure: usize,
    measure_samples: usize,
    generation: u64,
    rendered_mml_hash: u64,
    mml: String,
}

/// `u` で 1 回だけ取り消せる編集の、セル 1 つぶんの記録。
///
/// `p`（paste）は 1 セルだが、chord wizard（`G`）は chord 行・init セル・
/// カーソルセルを **1 操作で複数まとめて**書く。取り消しはその塊が単位なので、
/// [`crate::editor::DawEditorState::cell_undo`] は並びで持つ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NormalCellUndo {
    track: usize,
    measure: usize,
    /// 書き込む前の内容。取り消しはここへ戻す。
    previous: String,
    /// 書き込んだ内容。取り消し時にセルがこれのままのときだけ戻す
    /// （書いたあと手で直したセルを、取り消しで壊さないため）。
    written: String,
}

// ─── DawApp ───────────────────────────────────────────────────

pub struct DawApp {
    pub(crate) workspace_kind: WorkspaceKind,
    pub(crate) daily_page_date: Option<String>,
    pub(crate) config_app_dir: Option<std::path::PathBuf>,
    pub(crate) editor: DawEditorState,

    pub(crate) mode: DawMode,
    pub(crate) help_origin: DawMode,
    pub(crate) sound_check_guide: cmrt_tui_core::sound_check_guide::SoundCheckGuide,
    pub(crate) textarea: TextArea<'static>,

    cfg: Arc<Config>,
    /// カタログのプラグインごとのロード済み CLAP entry。
    /// render_server backend / テストでは空。
    plugin_entries: cmrt_offline_render::PluginEntries,

    /// セルごとのキャッシュ [track][measure]
    pub(crate) cache: Arc<Mutex<Vec<Vec<CellCache>>>>,

    /// キャッシュワーカースレッドへのジョブチャネル
    /// 設定数ワーカーで処理し、prepare 段階の排他は core-lib 側で行う
    cache_tx: std::sync::mpsc::Sender<CacheJob>,
    cache_render_workers: usize,
    render_queue: RenderQueue,

    pub(crate) playback: DawPlaybackRuntime,

    /// DAW モード下部に表示するデバッグログ。
    pub(crate) log_lines: Arc<Mutex<VecDeque<String>>>,

    /// track ごとの再レンダリング進捗ログ管理。
    track_rerender_batches: Arc<Mutex<Vec<Option<TrackRerenderBatch>>>>,

    /// playable track ごとの solo 状態。いずれかが true の間だけ solo モード。
    pub(crate) solo_tracks: Vec<bool>,
    /// playable track ごとの音量(dB)。
    pub(crate) track_volumes_db: Vec<i32>,
    /// Grid history を preview 無しで import した直後だけ立つ。先頭小節の cache が
    /// 出そろったら [`Self::pump_pending_auto_trim`] が初期音量を決めて下ろす。
    pub(crate) pending_auto_trim: bool,
    pub(crate) overlays: DawOverlays,
    pub(crate) patch_phrase_store: cmrt_history::PatchPhraseStore,
    pub(crate) patch_phrase_store_dirty: bool,
    pub(crate) random_patch_decks: cmrt_tui_core::random::RandomIndexDecks,

    /// chord wizard（`G`）が抽選するコード進行の供給元。未注入なら `None`
    /// （`G` は「カタログが空」と 1 行ログを出して何もしない）。
    pub(crate) chord_progression_source: Option<ChordProgressionSource>,

    /// app が起動時に file cache から読み込む、画面横断の patch catalog。
    ///
    /// DAW 自身は音色 file を走査しない（`app/src/tui/session.rs` の方針）。
    /// 一覧が要る場面は `patch_catalog` モジュール経由でここを読む。
    pub(crate) patch_load: Arc<Mutex<cmrt_tui_core::patch_load::PatchLoadState>>,

    /// 小節セルの MML を書くための 1 行入力オーバーレイ（`Ctrl+P`）。
    pub(crate) mml_overlay: cmrt_mml_overlay::MmlOverlay<'static>,
    /// オーバーレイの打鍵と chord wizard の即時試聴を鳴らす先。
    /// play server が無い構成では `None`（音が鳴らないだけで、編集は動く）。
    pub(crate) mml_overlay_sender: Option<cmrt_mml_overlay::MmlOverlaySender>,
}

impl DawApp {
    pub fn new(
        cfg: Arc<Config>,
        plugin_entries: cmrt_offline_render::PluginEntries,
        patch_load: Arc<Mutex<cmrt_tui_core::patch_load::PatchLoadState>>,
        realtime_play_supervisor: Option<Arc<cmrt_realtime_play::RealtimePlayServerSupervisor>>,
    ) -> Self {
        Self::new_for_workspace(
            cfg,
            plugin_entries,
            patch_load,
            realtime_play_supervisor,
            WorkspaceKind::Persistent,
        )
    }

    /// `patch_load` は app 側が起動時に立てた共有 catalog をそのまま渡す（`Arc::clone` 1 本）。
    /// DAW 内でもう 1 本立てると、file 走査を DAW だけがやり直すことになる。
    ///
    /// `realtime_play_supervisor` も同じく app の 1 本を渡す。MML オーバーレイは
    /// keyboard 画面と同じ音源 instance を借りるので、DAW 内で supervisor をもう 1 本
    /// 作ると SHM 接続が二重になる。DAW 自身の演奏用 supervisor をどこから得るかは
    /// backend で変わる（`init::realtime_audio_wiring`）。`play_server` backend は
    /// HTTP しか使わないので別に 1 本作り、`cache_player` backend は SHM の live 経路を
    /// 使うので**この注入された 1 本をそのまま共有する**。
    pub fn new_for_workspace(
        cfg: Arc<Config>,
        plugin_entries: cmrt_offline_render::PluginEntries,
        patch_load: Arc<Mutex<cmrt_tui_core::patch_load::PatchLoadState>>,
        realtime_play_supervisor: Option<Arc<cmrt_realtime_play::RealtimePlayServerSupervisor>>,
        workspace_kind: WorkspaceKind,
    ) -> Self {
        init::new(
            cfg,
            plugin_entries,
            patch_load,
            realtime_play_supervisor,
            workspace_kind,
        )
    }

    /// chord wizard（`G`）が使うコード進行カタログの供給元を注入する。
    ///
    /// 構築時ではなく後付けなのは、DAW 画面を作るたびにカタログの取得完了を
    /// 待たせないため（[`ChordProgressionSource`] を参照）。
    pub fn set_chord_progression_source(&mut self, source: ChordProgressionSource) {
        self.chord_progression_source = Some(source);
    }

    pub fn workspace_kind(&self) -> WorkspaceKind {
        self.workspace_kind
    }

    pub fn daily_page_date(&self) -> Option<&str> {
        self.daily_page_date.as_deref()
    }

    fn offline_render_available(&self) -> bool {
        self.plugin_entries.is_available()
            || self.cfg.offline_render_backend == cmrt_runtime::OfflineRenderBackend::RenderServer
    }

    pub(crate) fn ab_repeat_state(&self) -> AbRepeatState {
        *self.playback.ab_repeat.lock().unwrap()
    }

    // ─── 描画 ─────────────────────────────────────────────────

    fn draw(&self, f: &mut Frame) {
        ui::draw(self, f);
    }

    fn append_log_line(&self, message: impl Into<String>) {
        append_log_line(&self.log_lines, message);
    }
}

type LogSink = fn(&str);
static LOG_SINK: OnceLock<LogSink> = OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合、この crate のログはファイルへ残らない（画面下部の表示は変わらない）。
///
/// 直接ファイルへ書かないのは、書き先が実ユーザーの `log/log.txt` 固定で、
/// テストからでもそこへ追記してしまうため。他の画面 crate と同じ注入の形にそろえてある。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

/// 注入されたログ sink へ 1 行流す。画面表示用バッファを持たない場所（HTTP サーバー
/// スレッドや、まだ `DawApp` が組み上がっていない初期化中）はこちらを使う。
pub(crate) fn log_line(line: &str) {
    if let Some(sink) = LOG_SINK.get() {
        sink(line);
    }
}

/// DAW のログ 1 行を、注入されたログ sink と画面表示用バッファの両方へ流す。
pub(crate) fn append_log_line(
    log_lines: &Arc<Mutex<VecDeque<String>>>,
    message: impl Into<String>,
) {
    let line = message.into();
    log_line(&line);
    cmrt_tui_core::logging::append_log_line_in_memory(log_lines, line);
}

pub fn ensure_http_server_for_mode_switch() {
    http_server::ensure_daw_http_server_thread();
}

/// app 側テストから DAW への HTTP モード切替要求を注入するためのヘルパ。
/// crate をまたぐため `test-support` feature でもゲートする（app の dev-dependency で有効化）。
#[cfg(any(test, feature = "test-support"))]
pub fn request_http_mode_switch() {
    http_server::request_daw_mode_switch();
}

pub fn take_http_mode_switch_request() -> bool {
    http_server::take_daw_mode_switch_request()
}

#[cfg(test)]
mod tests;
