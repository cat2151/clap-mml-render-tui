use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use super::{AbRepeatState, DawPlayState, PlayPosition};

/// DAW の再生・preview セッションで共有される runtime 状態。
///
/// 編集グリッドやレンダーキューとは寿命と同期方法が異なるため、ひとまとまりにする。
pub(crate) struct DawPlaybackRuntime {
    pub(crate) play_state: Arc<Mutex<DawPlayState>>,
    pub(crate) transition_lock: Arc<Mutex<()>>,
    pub(crate) preview_session: Arc<AtomicU64>,
    pub(crate) preview_sink: Arc<Mutex<Option<Arc<rodio::Sink>>>>,
    pub(crate) realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
    pub(crate) position: Arc<Mutex<Option<PlayPosition>>>,
    pub(crate) ab_repeat: Arc<Mutex<AbRepeatState>>,
    pub(crate) overlay_preview_cache: Arc<Mutex<HashMap<u64, Arc<Vec<f32>>>>>,
    pub(crate) measure_mmls: Arc<Mutex<Vec<String>>>,
    pub(crate) measure_track_mmls: Arc<Mutex<Vec<Vec<String>>>>,
    pub(crate) measure_samples: Arc<Mutex<usize>>,
    pub(crate) track_gains: Arc<Mutex<Vec<f32>>>,
}

impl DawPlaybackRuntime {
    pub(crate) fn new(
        realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
        position: Arc<Mutex<Option<PlayPosition>>>,
        ab_repeat: Arc<Mutex<AbRepeatState>>,
        measure_mmls: Arc<Mutex<Vec<String>>>,
        measure_track_mmls: Arc<Mutex<Vec<Vec<String>>>>,
        track_gains: Arc<Mutex<Vec<f32>>>,
    ) -> Self {
        Self {
            play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
            transition_lock: Arc::new(Mutex::new(())),
            preview_session: Arc::new(AtomicU64::new(0)),
            preview_sink: Arc::new(Mutex::new(None)),
            realtime_play_server,
            position,
            ab_repeat,
            overlay_preview_cache: Arc::new(Mutex::new(HashMap::new())),
            measure_mmls,
            measure_track_mmls,
            measure_samples: Arc::new(Mutex::new(0)),
            track_gains,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(tracks: usize, measures: usize) -> Self {
        Self::new(
            None,
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(AbRepeatState::Off)),
            Arc::new(Mutex::new(vec![String::new(); measures])),
            Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures])),
            Arc::new(Mutex::new(vec![0.0; tracks])),
        )
    }
}
