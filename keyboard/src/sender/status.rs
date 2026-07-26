use std::time::Duration;

use cmrt_realtime_play::{PatchVoicing, VoicingReport};

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
    Detecting { previous: Option<VoicingReport> },
    Detected(VoicingReport),
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
    pub phase: KeyboardConnectionPhase,
    pub last_send: Option<Duration>,
    pub buffer_multiplier: u8,
    pub voicing: KeyboardVoicingStatus,
    pub voicing_patch: Option<String>,
}

impl Default for KeyboardConnectionStatus {
    fn default() -> Self {
        Self::new(4)
    }
}

impl KeyboardConnectionStatus {
    pub(super) fn new(buffer_multiplier: u8) -> Self {
        Self {
            phase: KeyboardConnectionPhase::Idle,
            last_send: None,
            buffer_multiplier,
            voicing: KeyboardVoicingStatus::Unavailable,
            voicing_patch: None,
        }
    }

    pub(super) fn begin_connecting(
        &mut self,
        buffer_multiplier: u8,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
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
