use std::{sync::Mutex, time::Duration};

use anyhow::Result;
use cmrt_realtime_play::VoicingReport;

use super::{
    KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardVoicingStatus, PatchRequest,
};

pub(super) fn set_result(
    status: &Mutex<KeyboardConnectionStatus>,
    buffer_multiplier: u8,
    result: Result<()>,
    elapsed: Option<Duration>,
    idle_on_success: bool,
) {
    let mut status = status.lock().unwrap();
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
    buffer_multiplier: u8,
    result: Result<Option<VoicingReport>>,
    request: PatchRequest<'_>,
    elapsed: Option<Duration>,
) {
    let mut status = status.lock().unwrap();
    status.buffer_multiplier = buffer_multiplier;
    status.last_send = elapsed;
    status.voicing_patch = request.patch.map(str::to_string);
    match result {
        Ok(Some(report)) => {
            status.phase = KeyboardConnectionPhase::Ready;
            status.voicing = KeyboardVoicingStatus::Detected(report);
        }
        Ok(None) => {
            status.phase = KeyboardConnectionPhase::Ready;
            status.voicing = request
                .known_voicing
                .map(KeyboardVoicingStatus::Cached)
                .unwrap_or(KeyboardVoicingStatus::Unavailable);
        }
        Err(error) => {
            status.phase = KeyboardConnectionPhase::Error(error.to_string());
            status.voicing = KeyboardVoicingStatus::Unavailable;
        }
    }
}
