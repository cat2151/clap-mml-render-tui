use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopGridChange {
    #[default]
    Initial,
    Random,
    BatchRandom,
    TrackOrder,
    Pad(char),
    Category,
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
    Quit,
}
