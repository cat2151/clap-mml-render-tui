use std::time::Duration;

use cmrt_realtime_play::{PatchVoicing, VoicingReport};

use crate::session_state::KeyboardTransport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyboardConnectionPhase {
    Idle,
    Connecting,
    PatchSetting,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyboardVoicingStatus {
    Unavailable,
    Detecting {
        previous: Option<VoicingReport>,
    },
    Detected(VoicingReport),
    /// file cache から復元した判定結果。probe を行っていない。
    Cached(PatchVoicing),
}

impl KeyboardVoicingStatus {
    pub fn effective_decision(&self) -> PatchVoicing {
        match self {
            Self::Detected(report)
            | Self::Detecting {
                previous: Some(report),
            } => report.decision,
            Self::Cached(voicing) => *voicing,
            Self::Unavailable | Self::Detecting { previous: None } => PatchVoicing::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyboardConnectionStatus {
    pub transport: KeyboardTransport,
    pub phase: KeyboardConnectionPhase,
    pub last_send: Option<Duration>,
    pub buffer_multiplier: u8,
    pub voicing: KeyboardVoicingStatus,
    /// `voicing` がどの patch に対する結果なのか。file cache への書き戻し時に、
    /// 連続切替で別 patch の結果を紐づけないための対応付けに使う。
    pub voicing_patch: Option<String>,
}

impl Default for KeyboardConnectionStatus {
    fn default() -> Self {
        Self::new(KeyboardTransport::SharedMemory, 4)
    }
}

impl KeyboardConnectionStatus {
    pub(super) fn new(transport: KeyboardTransport, buffer_multiplier: u8) -> Self {
        Self {
            transport,
            phase: KeyboardConnectionPhase::Idle,
            last_send: None,
            buffer_multiplier,
            voicing: KeyboardVoicingStatus::Unavailable,
            voicing_patch: None,
        }
    }

    pub(super) fn begin_connecting(
        &mut self,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        self.transport = transport;
        self.phase = KeyboardConnectionPhase::Connecting;
        self.last_send = None;
        self.buffer_multiplier = buffer_multiplier;
        self.begin_voicing(patch, known_voicing);
    }

    pub(super) fn begin_patch_setting(
        &mut self,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        self.phase = KeyboardConnectionPhase::PatchSetting;
        self.begin_voicing(patch, known_voicing);
    }

    /// cache ヒット時は probe を待たずに判定結果を確定させる。
    fn begin_voicing(&mut self, patch: Option<&str>, known_voicing: Option<PatchVoicing>) {
        self.voicing_patch = patch.map(str::to_string);
        if let Some(voicing) = known_voicing {
            self.voicing = KeyboardVoicingStatus::Cached(voicing);
            return;
        }
        let previous =
            match std::mem::replace(&mut self.voicing, KeyboardVoicingStatus::Unavailable) {
                KeyboardVoicingStatus::Detected(report) => Some(report),
                KeyboardVoicingStatus::Detecting { previous } => previous,
                KeyboardVoicingStatus::Cached(_) | KeyboardVoicingStatus::Unavailable => None,
            };
        self.voicing = KeyboardVoicingStatus::Detecting { previous };
    }
}
