use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use cmrt_realtime_play::RealtimePlayServerSupervisor;

use super::playback::live_gain::LiveTrackGain;
use super::playback::DawPlaybackStartupState;
use super::{AbRepeatState, DawPlayState, PlayPosition};

/// DAW の再生・preview セッションで共有される runtime 状態。
///
/// 編集グリッドやレンダーキューとは寿命と同期方法が異なるため、ひとまとまりにする。
pub(crate) struct DawPlaybackRuntime {
    pub(crate) play_state: Arc<Mutex<DawPlayState>>,
    pub(crate) transition_lock: Arc<Mutex<()>>,
    pub(crate) preview_session: Arc<AtomicU64>,
    pub(crate) preview_sink: Arc<Mutex<Option<Arc<rodio::Player>>>>,
    pub(crate) realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
    pub(crate) position: Arc<Mutex<Option<PlayPosition>>>,
    pub(crate) ab_repeat: Arc<Mutex<AbRepeatState>>,
    pub(crate) overlay_preview_cache: Arc<Mutex<HashMap<u64, Arc<Vec<f32>>>>>,
    pub(crate) measure_mmls: Arc<Mutex<Vec<String>>>,
    pub(crate) measure_track_mmls: Arc<Mutex<Vec<Vec<String>>>>,
    pub(crate) measure_samples: Arc<Mutex<usize>>,
    /// live mix の instance へ**最後に送った** gain。
    ///
    /// 差分だけ送るための記録なので、送っていない状態（＝演奏していない）は空。
    /// 演奏を止めるときに空へ戻すので、次の演奏では必ず全 track ぶん送り直す
    /// （そのあいだにサーバーが再起動していても gain が食い違わない）。
    pub(crate) live_track_gains: Arc<Mutex<Vec<LiveTrackGain>>>,
    /// 演奏を始めてから最初の音が出るまでの進み具合。演奏スレッドが書き、
    /// 描画スレッドが中央 overlay として読む。待っていないあいだは空。
    pub(crate) startup: DawPlaybackStartupState,
}

impl DawPlaybackRuntime {
    pub(crate) fn new(
        realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
        position: Arc<Mutex<Option<PlayPosition>>>,
        ab_repeat: Arc<Mutex<AbRepeatState>>,
        measure_mmls: Arc<Mutex<Vec<String>>>,
        measure_track_mmls: Arc<Mutex<Vec<Vec<String>>>>,
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
            live_track_gains: Arc::new(Mutex::new(Vec::new())),
            startup: DawPlaybackStartupState::default(),
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
        )
    }
}
