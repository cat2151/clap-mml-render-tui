use std::time::{Duration, Instant};

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
    /// play server の起動が何本目の CLAP instance まで進んだか `(済み, 総数)`。
    ///
    /// **数える主は supervisor**（子プロセスの stderr の
    /// `cmrt-server-startup: instances=N/M` を拾っている）。ここは
    /// [`super::KeyboardMidiSender::status`] が読んで詰め直すだけで、
    /// 自前では数えない。
    pub server_startup: Option<(usize, usize)>,
    /// いまの待ちが始まった時刻。経過秒の表示に使う。待っていなければ `None`。
    pub stage_started_at: Option<Instant>,
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
            server_startup: None,
            stage_started_at: None,
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
        self.server_startup = None;
        // 経過秒は**この時点から**通しで数える。段が変わるたびに 0 へ戻すと、
        // 「合わせて何秒待たされたか」が読めなくなる。
        self.stage_started_at = Some(Instant::now());
        self.begin_voicing(patch, known_voicing);
    }

    pub(super) fn begin_patch_setting(
        &mut self,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        self.phase = KeyboardConnectionPhase::PatchSetting;
        self.stage_started_at.get_or_insert_with(Instant::now);
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
