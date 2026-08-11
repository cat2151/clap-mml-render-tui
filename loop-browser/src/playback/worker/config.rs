use cmrt_tui_core::bpm::BpmMode;

use crate::LoopPlaybackGrid;

pub(in crate::playback) struct PlaybackWorkerConfig {
    pub(in crate::playback) grid: LoopPlaybackGrid,
    pub(in crate::playback) track_volumes_db: Vec<i32>,
    pub(in crate::playback) solo_tracks: Vec<bool>,
    pub(in crate::playback) bpm_mode: BpmMode,
}
