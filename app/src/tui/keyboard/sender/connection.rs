use std::{sync::Mutex, time::Duration};

use anyhow::Result;

use super::{KeyboardConnectionPhase, KeyboardConnectionStatus};
use crate::{fast_midi_ipc::FastMidiClient, history::KeyboardTransport};

pub(super) fn set_result(
    status: &Mutex<KeyboardConnectionStatus>,
    transport: KeyboardTransport,
    buffer_multiplier: u8,
    result: Result<()>,
    elapsed: Option<Duration>,
    idle_on_success: bool,
) {
    *status.lock().unwrap() = KeyboardConnectionStatus {
        transport,
        buffer_multiplier,
        phase: match result {
            Ok(()) if idle_on_success => KeyboardConnectionPhase::Idle,
            Ok(()) => KeyboardConnectionPhase::Ready,
            Err(error) => KeyboardConnectionPhase::Error(error.to_string()),
        },
        last_send: elapsed,
    };
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
