//! DAW モードの型定義

use std::sync::Arc;

// ─── キャッシュ ───────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum CacheState {
    Empty,     // MML が空
    Pending,   // MML あり、キャッシュ未作成
    Rendering, // キャッシュ生成中
    Ready,     // レンダリング済み
    Error,     // レンダリング失敗
}

/// セルごとのレンダリングキャッシュ。
///
/// `samples` フィールドはメモリ内にステレオ PCM サンプルを保持する。
/// 低 BPM では 1 小節が非常に長くなり得るため（BPM=1, 4/4, 44100 Hz → ~21M サンプル ≈ 85 MB/cell）、
/// キャッシュワーカーはサイズが [`MAX_CACHED_SAMPLES`] を超えるセルのサンプルを保持しない
/// （その場合 `state` は `Ready` だが `samples` は `None` のまま残り、再生時にフォールバックレンダリングされる）。
/// また、再 render 中の継続再生のため、`Pending` / `Rendering` でも直前世代の `samples` と
/// `rendered_measure_samples` を保持している場合がある。
///
/// [`MAX_CACHED_SAMPLES`]: super::MAX_CACHED_SAMPLES
#[derive(Clone)]
pub struct CellCache {
    pub(super) state: CacheState,
    /// レンダリング済みのステレオサンプル。
    ///
    /// 通常は `Ready` かつサイズ上限以内のときに `Some` だが、再 render 中の playback fallback 用に
    /// `Pending` / `Rendering` でも直前世代の `Some` を保持しうる。
    pub(super) samples: Option<Arc<Vec<f32>>>,
    /// `samples` が対応している「1 小節ぶん」のサンプル数。
    ///
    /// サンプル末尾の余韻により `samples.len()` はこの値より長いことがあるため、
    /// playback 側では `samples.len()` ではなくこの値で互換性を判定する。
    pub(super) rendered_measure_samples: Option<usize>,
    /// 現在セルに対して有効なレンダリング世代。
    pub(super) generation: u64,
    /// 現在 Ready な WAV を生成したレンダリング MML のハッシュ。
    pub(super) rendered_mml_hash: Option<u64>,
}

impl CellCache {
    pub(super) fn empty() -> Self {
        Self {
            state: CacheState::Empty,
            samples: None,
            rendered_measure_samples: None,
            generation: 0,
            rendered_mml_hash: None,
        }
    }

    /// セルを Pending に戻す。
    ///
    /// 再 render 中でも演奏を止めないため、旧世代の `samples` / `rendered_measure_samples` はここでは消さない。
    /// 新しい render が完了して `Ready` になるまで、playback fallback 用のステールキャッシュとして扱う。
    pub(super) fn set_pending(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
        }
        self.state = CacheState::Pending;
        self.rendered_mml_hash = None;
    }
}

// ─── 演奏状態 ─────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum DawPlayState {
    Idle,
    Playing,
    /// 一発プレビュー（ループなし）
    Preview,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AbRepeatState {
    #[default]
    Off,
    FixStart {
        start_measure_index: usize,
        end_measure_index: usize,
    },
    FixEnd {
        start_measure_index: usize,
        end_measure_index: usize,
    },
}

impl AbRepeatState {
    pub fn normalized_range(self, effective_count: usize) -> Option<(usize, usize)> {
        if effective_count == 0 {
            return None;
        }
        match self {
            Self::Off => None,
            Self::FixStart {
                start_measure_index,
                end_measure_index,
            }
            | Self::FixEnd {
                start_measure_index,
                end_measure_index,
            } => {
                let last_measure_index = effective_count - 1;
                let start_measure_index = start_measure_index.min(last_measure_index);
                let end_measure_index = end_measure_index.min(last_measure_index);
                Some((
                    start_measure_index.min(end_measure_index),
                    start_measure_index.max(end_measure_index),
                ))
            }
        }
    }

    pub fn marker_indices(self) -> Option<(usize, usize)> {
        match self {
            Self::Off => None,
            Self::FixStart {
                start_measure_index,
                end_measure_index,
            }
            | Self::FixEnd {
                start_measure_index,
                end_measure_index,
            } => Some((start_measure_index, end_measure_index)),
        }
    }
}

/// 再生中の小節・ビート位置
#[derive(Clone)]
pub struct PlayPosition {
    /// 0-based 小節インデックス（表示は +1）
    pub measure_index: usize,
    /// この小節の再生開始時刻
    pub measure_start: std::time::Instant,
    /// この小節の再生時間
    pub measure_duration: std::time::Duration,
}

// ─── 内部モード ───────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DawMode {
    Normal,
    Insert,
    Help,
    Mixer,
    History,
    PatchSelect,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DawHistoryPane {
    History,
    Favorites,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DawPatchSelectPane {
    Patches,
    Favorites,
}

/// handle_normal の戻り値
pub enum DawNormalAction {
    Continue,
    ReturnToTui,
    QuitApp,
}

/// DAW モード終了後の画面切替/終了通知
pub enum DawExitReason {
    /// n キーで notepad へ切り替える
    ReturnToTui,
    /// q キーでアプリを終了する
    QuitApp,
}
