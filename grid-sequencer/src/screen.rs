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
}

impl GridSequencerScreen {
    /// テスト用。既定のサンプルレートで作る。
    pub fn new(midi_sender: Option<GridMidiSender>) -> Self {
        Self::with_sample_rate(midi_sender, DEFAULT_SAMPLE_RATE)
    }

    pub fn with_sample_rate(midi_sender: Option<GridMidiSender>, sample_rate: f64) -> Self {
        Self {
            midi_sender,
            state: GridState::default(),
            sample_rate,
            help_open: false,
            patch_status: GridPatchStatus::default(),
            grid_ready: false,
        }
    }
}
