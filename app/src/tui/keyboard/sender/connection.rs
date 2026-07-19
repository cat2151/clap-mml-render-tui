use std::{sync::Mutex, time::Duration};

use anyhow::Result;

use super::{
    KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardVoicingStatus, PatchRequest,
};
use crate::realtime_play::VoicingReport;
use crate::{fast_midi_ipc::FastMidiClient, history::KeyboardTransport};

pub(super) fn set_result(
    status: &Mutex<KeyboardConnectionStatus>,
    transport: KeyboardTransport,
    buffer_multiplier: u8,
    result: Result<()>,
    elapsed: Option<Duration>,
    idle_on_success: bool,
) {
    let mut status = status.lock().unwrap();
    status.transport = transport;
    status.buffer_multiplier = buffer_multiplier;
    status.phase = match result {
        Ok(()) if idle_on_success => KeyboardConnectionPhase::Idle,
        Ok(()) => KeyboardConnectionPhase::Ready,
        Err(error) => KeyboardConnectionPhase::Error(error.to_string()),
    };
    status.last_send = elapsed;
}

pub(super) fn set_prepare_result(
    status: &Mutex<KeyboardConnectionStatus>,
    transport: KeyboardTransport,
    buffer_multiplier: u8,
    result: Result<Option<VoicingReport>>,
    request: PatchRequest<'_>,
    elapsed: Option<Duration>,
) {
    let mut status = status.lock().unwrap();
    status.transport = transport;
    status.buffer_multiplier = buffer_multiplier;
    status.last_send = elapsed;
    status.voicing_patch = request.patch.map(str::to_string);
    match result {
        Ok(Some(report)) => {
            status.phase = KeyboardConnectionPhase::Ready;
            status.voicing = KeyboardVoicingStatus::Detected(report);
        }
        // probe を省いた場合は cache の判定結果をそのまま維持する。
        Ok(None) => {
            status.phase = KeyboardConnectionPhase::Ready;
            status.voicing = match request.known_voicing {
                Some(voicing) => KeyboardVoicingStatus::Cached(voicing),
                None => KeyboardVoicingStatus::Unavailable,
            };
        }
        Err(error) => {
            status.phase = KeyboardConnectionPhase::Error(error.to_string());
            status.voicing = KeyboardVoicingStatus::Unavailable;
        }
    }
}

#[cfg(windows)]
pub(super) fn connect_fast_client(port: u16) -> Result<FastMidiClient> {
    let deadline = std::time::Instant::now() + Duration::from_millis(500);
    loop {
        match FastMidiClient::connect(port) {
            Ok(client) => return Ok(client),
            Err(error) if std::time::Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(not(windows))]
pub(super) fn connect_fast_client(port: u16) -> Result<FastMidiClient> {
    FastMidiClient::connect(port)
}
