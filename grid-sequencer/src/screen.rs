use std::time::Instant;

use super::{GridMidiSender, GridPatchStatus, GridState};

/// サンプルレート未指定時の既定値（config.toml の既定と同じ）。
const DEFAULT_SAMPLE_RATE: f64 = 48_000.0;

/// grid sequencer 画面が所有する接続・演奏・表示状態。
pub struct GridSequencerScreen {
    /// `None` ならテストモード（MIDI を送らない）。
    pub midi_sender: Option<GridMidiSender>,
    pub state: GridState,
    /// live MIDI の offset をフレーム数で組み立てるためのサンプルレート。
    /// realtime play server が使う config.toml の `sample_rate` と一致させること。
    pub(crate) sample_rate: f64,
    pub help_open: bool,
    /// 直近のランダム化時点での patch 一覧の状態。ステータス行の表示だけに使う。
    pub patch_status: GridPatchStatus,
    /// 一度でも grid を作ったか。2回目以降の入場で前回の grid を残すために見る。
    pub(crate) grid_ready: bool,
    /// chord mode を開始／継続できなかった理由。ステータス行に出す。
    pub(crate) chord_error: Option<String>,
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
    /// テスト用。既定のサンプルレートで作る。
    pub fn new(midi_sender: Option<GridMidiSender>) -> Self {
        Self::with_sample_rate(midi_sender, DEFAULT_SAMPLE_RATE)
    }

    pub fn with_sample_rate(midi_sender: Option<GridMidiSender>, sample_rate: f64) -> Self {
        Self::with_sample_rate_and_track_count(midi_sender, sample_rate, crate::GRID_ROWS)
    }

    pub fn with_track_count(midi_sender: Option<GridMidiSender>, track_count: usize) -> Self {
        Self::with_sample_rate_and_track_count(midi_sender, DEFAULT_SAMPLE_RATE, track_count)
    }

    pub fn with_sample_rate_and_track_count(
        midi_sender: Option<GridMidiSender>,
        sample_rate: f64,
        track_count: usize,
    ) -> Self {
        let track_count = cmrt_realtime_play::normalize_live_instance_count(track_count);
        Self {
            midi_sender,
            state: GridState::with_row_count(track_count),
            sample_rate,
            help_open: false,
            patch_status: GridPatchStatus::default(),
            grid_ready: false,
            chord_error: None,
            restart_notice: None,
            cycle_swap: None,
            waiting_for_patches: false,
            resume_at: None,
        }
    }

    /// ステータス行に出す chord mode の状態。
    pub fn chord_error(&self) -> Option<&str> {
        self.chord_error.as_deref()
    }

    /// 再起動アナウンスを表示中かどうか。
    pub fn restart_notice_open(&self) -> bool {
        self.restart_notice.is_some()
    }

    pub fn track_count(&self) -> usize {
        self.state.row_count()
    }
}
