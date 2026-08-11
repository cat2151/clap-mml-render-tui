use std::path::PathBuf;

use cmrt_tui_core::bpm::BpmMode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopGridChange {
    #[default]
    Initial,
    Random,
    BatchRandom,
    TrackOrder,
    Pad(char),
    Category,
    Tempo,
    /// オートランダムモードが 2 周ごとに自動で引き直したグリッド。
    AutoRandom,
}

impl LoopGridChange {
    pub fn label(self) -> String {
        match self {
            Self::Initial => "initial".to_string(),
            Self::Random => "random".to_string(),
            Self::BatchRandom => "batch-random".to_string(),
            Self::TrackOrder => "track-order".to_string(),
            Self::Pad(pad) => format!("pad-{pad}"),
            Self::Category => "category".to_string(),
            Self::Tempo => "tempo".to_string(),
            Self::AutoRandom => "auto-random".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopPlaybackClip {
    pub path: PathBuf,
    pub span_measures: usize,
    pub kind: cmrt_loop_domain::loop_wav_analysis::LoopWavKind,
    pub bpm: Option<f64>,
    pub category: Option<String>,
    pub meter_numerator: u16,
    pub meter_denominator: u16,
}

pub type LoopPlaybackGrid = Vec<Vec<Option<LoopPlaybackClip>>>;

pub enum LoopBrowserAction {
    Continue,
    Preview(PathBuf),
    Trigger {
        pad: char,
        path: PathBuf,
    },
    GridReplaced {
        start_measure: usize,
        grid: LoopPlaybackGrid,
        reason: LoopGridChange,
    },
    GridRefresh {
        grid: LoopPlaybackGrid,
        reason: LoopGridChange,
    },
    /// 演奏を止めずに裏で準備し、周の境目で差し替えてもらうグリッド（オートランダム）。
    GridPreload {
        grid: LoopPlaybackGrid,
        token: u64,
        reason: LoopGridChange,
    },
    TrackLayoutChanged {
        start_measure: usize,
        grid: LoopPlaybackGrid,
        track_volumes_db: Vec<i32>,
        solo_tracks: Vec<bool>,
    },
    TrackVolumeChanged {
        track: usize,
        volume_db: i32,
    },
    TrackSoloChanged {
        solo_tracks: Vec<bool>,
    },
    SetPlaybackPaused {
        paused: bool,
        start_measure: usize,
    },
    BpmChanged {
        mode: BpmMode,
        grid: LoopPlaybackGrid,
    },
    Quit,
}
