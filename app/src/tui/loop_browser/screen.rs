use super::{playback::LoopPlaybackController, LoopBrowser};

/// Loop Browser画面の編集状態とplayback runtimeの所有境界。
#[derive(Default)]
pub(in crate::tui) struct LoopBrowserScreen {
    pub(in crate::tui) state: LoopBrowser,
    pub(in crate::tui) playback: Option<LoopPlaybackController>,
}
