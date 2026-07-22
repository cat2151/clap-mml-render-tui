use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};

use crate::realtime_play::RealtimePlayServerSupervisor;

use super::{PlayState, TuiRenderQueue};

/// notepad（メイン）画面の再生セッションで共有される runtime 状態。
///
/// 編集バッファやオーバーレイ状態とは寿命・同期方法が異なるため、ひとまとまりにする。
/// セッション世代トークン（`playback_session`）・現在の sink・`play_state` を1型へ閉じ込め、
/// 「古いセッションの音が最新の再生状態を上書きする」レースの温床を局所化する。
pub(in crate::tui) struct TuiPlaybackRuntime {
    pub(in crate::tui) play_state: Arc<Mutex<PlayState>>,
    pub(in crate::tui) playback_session: Arc<AtomicU64>,
    pub(in crate::tui) active_sink: Arc<Mutex<Option<Arc<rodio::Sink>>>>,
    pub(in crate::tui) realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
    pub(in crate::tui) render_queue: TuiRenderQueue,
    pub(in crate::tui) active_offline_render_count: Arc<AtomicUsize>,
}

impl TuiPlaybackRuntime {
    pub(in crate::tui) fn new(
        realtime_play_server: Option<Arc<RealtimePlayServerSupervisor>>,
        render_queue: TuiRenderQueue,
        active_offline_render_count: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            play_state: Arc::new(Mutex::new(PlayState::Idle)),
            playback_session: Arc::new(AtomicU64::new(0)),
            active_sink: Arc::new(Mutex::new(None)),
            realtime_play_server,
            render_queue,
            active_offline_render_count,
        }
    }
}
