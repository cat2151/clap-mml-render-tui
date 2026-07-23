use super::{playback::LoopPlaybackController, LoopBrowser};

/// Loop Browser画面の編集状態とplayback runtimeの所有境界。
#[derive(Default)]
pub struct LoopBrowserScreen {
    pub state: LoopBrowser,
    pub playback: Option<LoopPlaybackController>,
}
