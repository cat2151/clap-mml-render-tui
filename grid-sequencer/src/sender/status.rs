use std::time::Duration;

use cmrt_tui_core::keyboard_session_state::KeyboardTransport;

/// realtime play server との接続状態。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridConnectionPhase {
    Idle,
    Connecting,
    PatchSetting,
    Ready,
    Error(String),
}

impl GridConnectionPhase {
    /// ノートを送ってよい状態か。Ready 以外では鳴らさず、note off も溜めない。
    pub fn accepts_notes(&self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Connecting => "connecting",
            Self::PatchSetting => "patch",
            Self::Ready => "ready",
            Self::Error(message) => message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridConnectionStatus {
    pub transport: KeyboardTransport,
    pub phase: GridConnectionPhase,
    pub last_send: Option<Duration>,
    /// 現在サーバーへ適用している patch。行0の patch と一致する。
    pub patch: Option<String>,
}

impl Default for GridConnectionStatus {
    fn default() -> Self {
        Self {
            transport: KeyboardTransport::SharedMemory,
            phase: GridConnectionPhase::Idle,
            last_send: None,
            patch: None,
        }
    }
}

impl GridConnectionStatus {
    pub(super) fn new(transport: KeyboardTransport) -> Self {
        Self {
            transport,
            ..Self::default()
        }
    }

    pub(super) fn begin_connecting(&mut self, patch: Option<&str>) {
        self.phase = GridConnectionPhase::Connecting;
        self.last_send = None;
        self.patch = patch.map(str::to_string);
    }

    pub(super) fn begin_patch_setting(&mut self, patch: Option<&str>) {
        self.phase = GridConnectionPhase::PatchSetting;
        self.patch = patch.map(str::to_string);
    }

    /// ワーカーでの実行結果を反映する。`idle_on_success` は停止コマンド用。
    pub(super) fn apply_result(
        &mut self,
        result: anyhow::Result<()>,
        elapsed: Option<Duration>,
        idle_on_success: bool,
    ) {
        self.phase = match result {
            Ok(()) if idle_on_success => GridConnectionPhase::Idle,
            Ok(()) => GridConnectionPhase::Ready,
            Err(error) => GridConnectionPhase::Error(error.to_string()),
        };
        self.last_send = elapsed;
    }
}

#[cfg(test)]
mod tests;
