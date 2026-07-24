use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use cmrt_tui_core::playback_session::PlaybackSession;

use super::TuiRenderQueue;

/// notepad（メイン）画面の再生セッションで共有される runtime 状態。
///
/// 編集バッファやオーバーレイ状態とは寿命・同期方法が異なるため、ひとまとまりにする。
/// セッション世代トークン・現在の sink・`play_state` は画面横断で共有するため
/// `cmrt-tui-core` の [`PlaybackSession`] が持ち、ここはそれと notepad 専用の
/// レンダリングキューを束ねる。
pub(crate) struct TuiPlaybackRuntime {
    pub(crate) session: PlaybackSession,
    pub(crate) render_queue: TuiRenderQueue,
    pub(crate) active_offline_render_count: Arc<AtomicUsize>,
}

impl TuiPlaybackRuntime {
    pub(crate) fn new(
        session: PlaybackSession,
        render_queue: TuiRenderQueue,
        active_offline_render_count: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            session,
            render_queue,
            active_offline_render_count,
        }
    }
}
