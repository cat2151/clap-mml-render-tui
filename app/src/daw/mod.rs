//! DAW 風モード
//!
//! 9 tracks × (0..=8 measures) の matrix
//!   measure 0 = 音色 (timbre) / track ごとの共通ヘッダ
//!   track   0 = 拍子JSON + テンポ (例: `{"beat": "4/4"}t120`) → render 時に全小節の先頭にくっつける
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
//!   p      : 演奏 / 停止 toggle
//!   Enter / Space       : 非play時、現在 track の現在 meas を一発再生
//!   Shift+Enter / Space : 非play時、現在 meas の全 track を一発再生
//!   Shift+P             : 非play時、現在 meas から演奏開始して継続
//!   s      : 現在 track の solo toggle
//!   r      : measure 0 にランダム音色を設定
//!   K / ?  : ヘルプ表示
//!   q      : アプリ終了
//!   n      : DAW モード終了 → TUI に戻る
//!   ESC    : 反応なし
//!
//! キー操作 (MIXER):
//!   h/l    : track 移動
//!   j/k    : volume -/+3dB
//!   ESC    : overlay を閉じる → NORMAL
//!
//! キー操作 (HISTORY):
//!   h/l・←/→ : History/Favorites ペイン切り替え
//!   j/k      : 行移動
//!   Enter    : 選択内容を現在 track/meas に適用
//!   ESC      : overlay を閉じる → NORMAL
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
mod input;
mod mml;
mod playback;
mod playback_util;
mod preview;
mod runtime;
mod save;
mod timing;
mod types;
mod ui;
mod wav_io;

use clack_host::prelude::PluginEntry;
use cmrt_core::{collect_patches, ensure_daw_dir, mml_render_for_cache, to_relative, write_wav};
use ratatui::Frame;
use tui_textarea::TextArea;

use std::collections::VecDeque;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use crate::config::Config;

// ─── 再エクスポート ───────────────────────────────────────────

use batch_logging::{TrackRerenderBatch, TrackRerenderBatchCompletionContext};
pub use types::DawExitReason;
pub(super) use types::{
    AbRepeatState, CacheState, CellCache, DawHistoryPane, DawMode, DawNormalAction, DawPlayState,
    PlayPosition,
};

// ─── 定数 ─────────────────────────────────────────────────────

/// track 数（固定）。track 0 = Tempo、track 1..=8 = 演奏 track。
pub const TRACKS: usize = 9;
/// 小節数（固定）。measure 0 = 音色列。measure 1..=MEASURES = 通常小節。
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

#[derive(Clone)]
pub(super) struct CacheJob {
    track: usize,
    measure: usize,
    measure_samples: usize,
    generation: u64,
    rendered_mml_hash: u64,
    mml: String,
}

// ─── DawApp ───────────────────────────────────────────────────

pub struct DawApp {
    /// data[track][measure]: track 0..tracks, measure 0..=measures
    pub(super) data: Vec<Vec<String>>,

    pub(super) cursor_track: usize,   // 0..tracks-1
    pub(super) cursor_measure: usize, // 0..=measures  (0 = 音色列)

    pub(super) mode: DawMode,
    pub(super) textarea: TextArea<'static>,

    cfg: Arc<Config>,
    entry_ptr: usize, // *const PluginEntry as usize (main() に生存保証)

    /// config から読み込んだトラック数（track 0 = ヘッダ/テンポ、track 1.. = 演奏トラック）
    pub(super) tracks: usize,
    /// config から読み込んだ小節数（measure 0 = 音色列、measure 1.. = 通常小節）
    pub(super) measures: usize,

    /// セルごとのキャッシュ [track][measure]
    pub(super) cache: Arc<Mutex<Vec<Vec<CellCache>>>>,

    /// キャッシュワーカースレッドへのジョブチャネル
    /// シリアルな単一ワーカーで処理することでファイル書き込みの競合を防ぐ
    cache_tx: std::sync::mpsc::Sender<CacheJob>,

    /// `mml_render_for_cache` の排他実行ロック。
    /// `mml_str_to_smf_bytes` が書き出す共有デバッグファイル
    /// （`pass1_tokens.json` など）を同時に書き込まないよう、
    /// `mml_render_for_cache` 呼び出し前に必ずこのロックを取得すること。
    render_lock: Arc<Mutex<()>>,

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
    pub(super) patch_phrase_store: crate::history::PatchPhraseStore,
    pub(super) history_overlay_patch_name: Option<String>,
    pub(super) history_overlay_history_cursor: usize,
    pub(super) history_overlay_favorites_cursor: usize,
    pub(super) history_overlay_focus: DawHistoryPane,
}

impl DawApp {
    pub fn new(cfg: Arc<Config>, entry_ptr: usize) -> Self {
        let tracks = cfg.daw_tracks.clamp(2, 64);
        let measures = cfg.daw_measures.clamp(1, 64);
        let mut data = vec![vec![String::new(); measures + 1]; tracks];
        // track 0 のデフォルトは拍子指定 JSON + テンポ設定
        data[0][0] = DEFAULT_TRACK0_MML.to_string();

        let cache = Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); measures + 1];
            tracks
        ]));

        // シリアルなキャッシュワーカースレッドを起動する。
        // チャネルが送信側（cache_tx）を介してジョブを受け取り順次レンダリングすることで
        // ファイル書き込み（clap-mml-render-tui/pass1_tokens.json 等）の競合と過剰スレッド生成を防ぐ。
        let (cache_tx, cache_rx) = std::sync::mpsc::channel::<CacheJob>();

        // `mml_render_for_cache` はキャッシュワーカーと再生スレッドの両方から呼ばれるため、
        // `mml_str_to_smf_bytes` が書き出す共有デバッグファイル
        // （`pass1_tokens.json` など）への同時書き込みを防ぐ排他ロックを共有する。
        let render_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
        let log_lines = Arc::new(Mutex::new(crate::logging::load_log_lines()));
        let track_rerender_batches = Arc::new(Mutex::new(vec![None; tracks]));
        let play_position = Arc::new(Mutex::new(None));
        let ab_repeat = Arc::new(Mutex::new(AbRepeatState::Off));
        let play_measure_mmls = Arc::new(Mutex::new(vec![String::new(); measures]));
        let play_measure_track_mmls =
            Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures]));
        let play_track_gains = Arc::new(Mutex::new(vec![0.0; tracks]));

        {
            let cache_worker = Arc::clone(&cache);
            let cfg_worker = Arc::clone(&cfg);
            let render_lock_worker = Arc::clone(&render_lock);
            let log_lines_worker = Arc::clone(&log_lines);
            let track_rerender_batches_worker = Arc::clone(&track_rerender_batches);
            let play_position_worker = Arc::clone(&play_position);
            let ab_repeat_worker = Arc::clone(&ab_repeat);
            let play_measure_mmls_worker = Arc::clone(&play_measure_mmls);
            let cache_tx_worker = cache_tx.clone();
            std::thread::spawn(move || {
                // SAFETY: entry は main() のスタックに生存している
                let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
                let daw_cfg = (*cfg_worker).clone();
                let rerender_completion_ctx = TrackRerenderBatchCompletionContext {
                    batches: Arc::clone(&track_rerender_batches_worker),
                    log_lines: Arc::clone(&log_lines_worker),
                    cache: Arc::clone(&cache_worker),
                    play_position: Arc::clone(&play_position_worker),
                    ab_repeat: Arc::clone(&ab_repeat_worker),
                    play_measure_mmls: Arc::clone(&play_measure_mmls_worker),
                    cache_tx: cache_tx_worker.clone(),
                };

                for job in cache_rx {
                    let track = job.track;
                    let measure = job.measure;
                    let mut skipped_stale_job = false;
                    {
                        let mut cache = cache_worker.lock().unwrap();
                        let cell = &mut cache[track][measure];
                        if cell.state == CacheState::Empty || cell.generation != job.generation {
                            skipped_stale_job = true;
                        } else {
                            cell.state = CacheState::Rendering;
                            cell.rendered_mml_hash = None;
                        }
                    }
                    if skipped_stale_job {
                        Self::complete_track_rerender_batch_measure(
                            &rerender_completion_ctx,
                            track,
                            measure,
                        );
                        continue;
                    }
                    let _guard = render_lock_worker.lock().unwrap();
                    let core_cfg = cmrt_core::CoreConfig::from(&daw_cfg);
                    match mml_render_for_cache(&job.mml, &core_cfg, entry_ref) {
                        Ok(samples) => {
                            let mut should_complete_batch = false;
                            {
                                let mut cache = cache_worker.lock().unwrap();
                                if cache[track][measure].generation != job.generation {
                                    skipped_stale_job = true;
                                } else {
                                    // 開発用: track/measure ごとに WAV ファイルを出力する
                                    // measure 0 は音色/ヘッダセルであり演奏内容ではないためスキップ
                                    let wav_ok = if measure > 0 {
                                        if let Ok(daw_dir) = ensure_daw_dir() {
                                            let wav_path = daw_dir.join(format!(
                                                "track{}_meas{}.wav",
                                                track, measure
                                            ));
                                            write_wav(
                                                &samples,
                                                daw_cfg.sample_rate as u32,
                                                &wav_path,
                                            )
                                            .is_ok()
                                        } else {
                                            false
                                        }
                                    } else {
                                        true
                                    };
                                    // WAV 書き出し失敗はデバッグ出力の問題であり、レンダリング自体は成功している。
                                    // そのため WAV 失敗時は Error としてユーザーに通知する。
                                    cache[track][measure].state = if wav_ok {
                                        CacheState::Ready
                                    } else {
                                        CacheState::Error
                                    };
                                    cache[track][measure].rendered_mml_hash = if wav_ok {
                                        Some(job.rendered_mml_hash)
                                    } else {
                                        None
                                    };
                                    // Ready かつサイズ上限以内のときのみサンプルをメモリに保持する。
                                    // 上限超過（低 BPM 等）や WAV 失敗時はサンプルを保持しない。
                                    if wav_ok && samples.len() <= MAX_CACHED_SAMPLES {
                                        cache[track][measure].samples = Some(Arc::new(samples));
                                        cache[track][measure].rendered_measure_samples =
                                            Some(job.measure_samples);
                                    } else {
                                        cache[track][measure].samples = None;
                                        cache[track][measure].rendered_measure_samples = None;
                                    }
                                    should_complete_batch = true;
                                }
                            }
                            if skipped_stale_job || should_complete_batch {
                                Self::complete_track_rerender_batch_measure(
                                    &rerender_completion_ctx,
                                    track,
                                    measure,
                                );
                            }
                        }
                        Err(_) => {
                            let mut should_complete_batch = false;
                            {
                                let mut cache = cache_worker.lock().unwrap();
                                if cache[track][measure].generation != job.generation {
                                    skipped_stale_job = true;
                                } else {
                                    cache[track][measure].state = CacheState::Error;
                                    // エラー時は古いサンプルを保持しない（ステールデータの排除）
                                    cache[track][measure].samples = None;
                                    cache[track][measure].rendered_measure_samples = None;
                                    cache[track][measure].rendered_mml_hash = None;
                                    should_complete_batch = true;
                                }
                            }
                            if skipped_stale_job || should_complete_batch {
                                Self::complete_track_rerender_batch_measure(
                                    &rerender_completion_ctx,
                                    track,
                                    measure,
                                );
                            }
                        }
                    }
                }
            });
        }

        let mut app = Self {
            data,
            cursor_track: 0,
            cursor_measure: 0,
            mode: DawMode::Normal,
            textarea: TextArea::default(),
            cfg,
            entry_ptr,
            tracks,
            measures,
            cache,
            cache_tx,
            render_lock,
            play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
            play_transition_lock: Arc::new(Mutex::new(())),
            preview_session: Arc::new(AtomicU64::new(0)),
            preview_sink: Arc::new(Mutex::new(None)),
            play_position,
            ab_repeat,
            play_measure_mmls,
            play_measure_track_mmls,
            play_measure_samples: Arc::new(Mutex::new(0)),
            log_lines,
            track_rerender_batches,
            solo_tracks: vec![false; tracks],
            track_volumes_db: vec![0; tracks],
            mixer_cursor_track: FIRST_PLAYABLE_TRACK.min(tracks - 1),
            play_track_gains,
            patch_phrase_store: crate::history::load_patch_phrase_store(),
            history_overlay_patch_name: None,
            history_overlay_history_cursor: 0,
            history_overlay_favorites_cursor: 0,
            history_overlay_focus: DawHistoryPane::History,
        };

        app.load();
        app.append_log_line("=== DAW mode ready ===");
        app
    }

    pub(super) fn ab_repeat_state(&self) -> AbRepeatState {
        *self.ab_repeat.lock().unwrap()
    }

    // ─── ランダム音色 ─────────────────────────────────────────

    fn pick_random_patch_name(&self) -> Option<String> {
        let dir = self.cfg.patches_dir.as_deref()?;
        let patches = collect_patches(dir).ok()?;
        if patches.is_empty() {
            return None;
        }
        use std::time::{SystemTime, UNIX_EPOCH};
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0) as usize;
        let idx = ns % patches.len();
        Some(to_relative(dir, &patches[idx]))
    }

    pub(super) fn solo_mode_active(&self) -> bool {
        self.solo_tracks
            .iter()
            .enumerate()
            .skip(FIRST_PLAYABLE_TRACK)
            .any(|(_, &is_solo)| is_solo)
    }

    pub(super) fn track_is_soloed(&self, track: usize) -> bool {
        self.solo_tracks.get(track).copied().unwrap_or(false)
    }

    pub(super) fn track_is_audible(&self, track: usize) -> bool {
        if track < FIRST_PLAYABLE_TRACK || !self.solo_mode_active() {
            return true;
        }
        self.track_is_soloed(track)
    }

    pub(super) fn track_volume_db(&self, track: usize) -> i32 {
        self.track_volumes_db.get(track).copied().unwrap_or(0)
    }

    pub(super) fn adjust_track_volume_db(&mut self, track: usize, delta_db: i32) -> bool {
        let Some(volume_db) = self.track_volumes_db.get_mut(track) else {
            return false;
        };
        let next = (*volume_db + delta_db).clamp(MIXER_MIN_DB, MIXER_MAX_DB);
        if next == *volume_db {
            return false;
        }
        *volume_db = next;
        true
    }

    pub(super) fn playback_track_gains(&self) -> Vec<f32> {
        (0..self.tracks)
            .map(|track| {
                if track < FIRST_PLAYABLE_TRACK || !self.track_is_audible(track) {
                    0.0
                } else {
                    10.0f32.powf(self.track_volume_db(track) as f32 / 20.0)
                }
            })
            .collect()
    }

    // ─── 描画 ─────────────────────────────────────────────────

    fn draw(&self, f: &mut Frame) {
        ui::draw(self, f);
    }

    fn append_log_line(&self, message: impl Into<String>) {
        crate::logging::append_log_line(&self.log_lines, message);
    }
}

#[cfg(test)]
#[path = "../tests/daw/mod.rs"]
mod tests;
