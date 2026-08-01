use std::time::Instant;

use super::{GridMidiSender, GridPatchStatus, GridState};

/// サンプルレート未指定時の既定値（config.toml の既定と同じ）。
const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;
/// 出力バッファ1単位のフレーム数の既定値（config.toml の既定と同じ）。
const DEFAULT_BUFFER_FRAMES: usize = 512;

/// 画面を組み立てるときに共有ランタイムから受け取る設定一式。
///
/// 引数が増えても呼び出し側が読めるように、`NotepadScreenParts` と同じく構造体で渡す。
pub struct GridSequencerParts {
    /// `None` ならテストモード（MIDI を送らない）。
    pub midi_sender: Option<GridMidiSender>,
    /// live MIDI の offset をフレーム数で組み立てるためのサンプルレート。
    /// realtime play server が使う config.toml の `sample_rate` と一致させること。
    pub sample_rate: f64,
    /// 出力バッファ1単位のフレーム数（config.toml の `buffer_size`）。
    /// 想定レイテンシの表示にだけ使う。これも server 側の設定と一致させること。
    pub buffer_frames: usize,
    pub track_count: usize,
    /// 前回終了時の chord mode。true なら patch 一覧が揃い次第 on にする。
    pub chord_enabled: bool,
}

impl Default for GridSequencerParts {
    fn default() -> Self {
        Self {
            midi_sender: None,
            sample_rate: DEFAULT_SAMPLE_RATE,
            buffer_frames: DEFAULT_BUFFER_FRAMES,
            track_count: crate::GRID_ROWS,
            chord_enabled: false,
        }
    }
}

/// grid sequencer 画面が所有する接続・演奏・表示状態。
pub struct GridSequencerScreen {
    /// `None` ならテストモード（MIDI を送らない）。
    pub midi_sender: Option<GridMidiSender>,
    pub state: GridState,
    /// live MIDI の offset をフレーム数で組み立てるためのサンプルレート。
    /// realtime play server が使う config.toml の `sample_rate` と一致させること。
    pub(crate) sample_rate: f64,
    /// 出力バッファ1単位のフレーム数。想定レイテンシの表示にだけ使う。
    pub(crate) buffer_frames: usize,
    pub help_open: bool,
    /// 直近のランダム化時点での patch 一覧の状態。ステータス行の表示だけに使う。
    pub patch_status: GridPatchStatus,
    /// 一度でも grid を作ったか。2回目以降の入場で前回の grid を残すために見る。
    pub(crate) grid_ready: bool,
    /// chord mode を開始／継続できなかった理由。コード進行行に出す。
    pub(crate) chord_error: Option<String>,
    /// chord mode が on か。セッションへ保存する値であり、[`GridState`] ではなく
    /// 画面側が持つ。`t` キーは `state` を作り直すので、そこに置くと切替のたびに落ちる。
    pub(crate) chord_enabled: bool,
    /// セッションから復元した chord mode を、patch 一覧が揃ってから適用するための予約。
    pub(crate) pending_chord: bool,
    /// コード進行データ更新の再起動アナウンスを出し始めた時刻。
    pub(crate) restart_notice: Option<Instant>,
    /// 待機 bank への先読みロードの進み具合。`None` なら先読みしていない。
    pub(crate) cycle_swap: Option<crate::cycle_swap::CycleSwap>,
    /// 音色ロードの完了を待っている。詳細は [`crate::start_wait`]。
    pub(crate) waiting_for_patches: bool,
    /// `Ready` へ戻ってから鳴らし始める時刻。待ちに入った時点では未定。
    pub(crate) resume_at: Option<Instant>,
}

impl GridSequencerScreen {
    /// テスト用。既定の設定で作る。
    pub fn new(midi_sender: Option<GridMidiSender>) -> Self {
        Self::new_with(GridSequencerParts {
            midi_sender,
            ..GridSequencerParts::default()
        })
    }

    /// テスト用。track 数だけ指定して作る。
    pub fn with_track_count(midi_sender: Option<GridMidiSender>, track_count: usize) -> Self {
        Self::new_with(GridSequencerParts {
            midi_sender,
            track_count,
            ..GridSequencerParts::default()
        })
    }

    pub fn new_with(parts: GridSequencerParts) -> Self {
        let GridSequencerParts {
            midi_sender,
            sample_rate,
            buffer_frames,
            track_count,
            chord_enabled,
        } = parts;
        let track_count = cmrt_realtime_play::normalize_live_instance_count(track_count);
        Self {
            midi_sender,
            state: GridState::with_row_count(track_count),
            sample_rate,
            buffer_frames,
            help_open: false,
            patch_status: GridPatchStatus::default(),
            grid_ready: false,
            chord_error: None,
            chord_enabled,
            pending_chord: chord_enabled,
            restart_notice: None,
            cycle_swap: None,
            waiting_for_patches: false,
            resume_at: None,
        }
    }

    /// コード進行行に出す chord mode のエラー。
    pub fn chord_error(&self) -> Option<&str> {
        self.chord_error.as_deref()
    }

    /// chord mode が on か。セッションへ保存する値。
    pub fn chord_enabled(&self) -> bool {
        self.chord_enabled
    }

    /// 想定レイテンシ（出力バッファに溜める目標時間）。
    ///
    /// サーバーはリングを `buffer_frames * multiplier` フレームまで溜めてから出すので、
    /// 送ってから鳴るまでの遅れはおおよそこの長さになる。
    pub(crate) fn buffer_latency_ms(&self, multiplier: u16) -> f64 {
        if self.sample_rate <= 0.0 {
            return 0.0;
        }
        (self.buffer_frames * usize::from(multiplier)) as f64 / self.sample_rate * 1000.0
    }

    /// 再起動アナウンスを表示中かどうか。
    pub fn restart_notice_open(&self) -> bool {
        self.restart_notice.is_some()
    }

    pub fn track_count(&self) -> usize {
        self.state.row_count()
    }
}
