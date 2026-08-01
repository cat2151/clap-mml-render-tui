use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

use anyhow::Result;
use cmrt_realtime_play::{PatchVoicing, RealtimePlayServerSupervisor, VoicingReport};

mod connection;
mod status;

use connection::{set_prepare_result, set_result};
pub use status::{KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardVoicingStatus};

const KEYBOARD_INSTANCE: u8 = 0;

#[derive(Clone, Copy, Debug)]
pub(super) struct PatchRequest<'a> {
    pub(super) patch: Option<&'a str>,
    pub(super) known_voicing: Option<PatchVoicing>,
}

enum KeyboardMidiCommand {
    Send {
        messages: Vec<[u8; 3]>,
    },
    Stop,
    Prepare {
        buffer_multiplier: u8,
        patch: Option<String>,
        known_voicing: Option<PatchVoicing>,
    },
    SetBufferMultiplier(u8),
    SetPatch {
        note_offs: Vec<[u8; 3]>,
        patch: Option<String>,
        known_voicing: Option<PatchVoicing>,
    },
    Shutdown,
}

pub struct KeyboardMidiSender {
    tx: mpsc::Sender<KeyboardMidiCommand>,
    status: Arc<Mutex<KeyboardConnectionStatus>>,
    worker: Option<JoinHandle<()>>,
}

impl KeyboardMidiSender {
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>, buffer_multiplier: u8) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(KeyboardConnectionStatus::new(buffer_multiplier)));
        let worker_status = Arc::clone(&status);
        let worker = std::thread::Builder::new()
            .name("keyboard-midi-sender".to_string())
            .spawn(move || run_midi_sender(rx, supervisor, worker_status, buffer_multiplier))
            .expect("keyboard MIDI sender thread should start");
        Self {
            tx,
            status,
            worker: Some(worker),
        }
    }

    pub fn send(&self, messages: Vec<[u8; 3]>, _patch: Option<&str>) {
        let _ = self.tx.send(KeyboardMidiCommand::Send { messages });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(KeyboardMidiCommand::Stop);
    }

    pub fn prepare(
        &self,
        buffer_multiplier: u8,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        self.status
            .lock()
            .unwrap()
            .begin_connecting(buffer_multiplier, patch, known_voicing);
        let _ = self.tx.send(KeyboardMidiCommand::Prepare {
            buffer_multiplier,
            patch: patch.map(str::to_string),
            known_voicing,
        });
    }

    pub fn set_buffer_multiplier(&self, multiplier: u8) {
        let _ = self
            .tx
            .send(KeyboardMidiCommand::SetBufferMultiplier(multiplier));
    }

    pub fn set_patch(
        &self,
        note_offs: Vec<[u8; 3]>,
        _previous_patch: Option<&str>,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        self.status
            .lock()
            .unwrap()
            .begin_patch_setting(patch, known_voicing);
        let _ = self.tx.send(KeyboardMidiCommand::SetPatch {
            note_offs,
            patch: patch.map(str::to_string),
            known_voicing,
        });
    }

    pub fn status(&self) -> KeyboardConnectionStatus {
        self.status.lock().unwrap().clone()
    }
}

impl Drop for KeyboardMidiSender {
    fn drop(&mut self) {
        let _ = self.tx.send(KeyboardMidiCommand::Stop);
        let _ = self.tx.send(KeyboardMidiCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_midi_sender(
    rx: mpsc::Receiver<KeyboardMidiCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<KeyboardConnectionStatus>>,
    initial_buffer_multiplier: u8,
) {
    let mut buffer_multiplier = initial_buffer_multiplier;
    let _ = supervisor.remember_live_buffer_multiplier(u16::from(buffer_multiplier));
    while let Ok(command) = rx.recv() {
        match command {
            KeyboardMidiCommand::Send { messages } => {
                let started = Instant::now();
                let result = supervisor
                    .send_midi(KEYBOARD_INSTANCE, &messages)
                    .map(|_| ());
                set_result(
                    &status,
                    buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    false,
                );
            }
            KeyboardMidiCommand::Stop => {
                let started = Instant::now();
                let result = supervisor.stop_live_instance(KEYBOARD_INSTANCE);
                set_result(
                    &status,
                    buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    true,
                );
            }
            KeyboardMidiCommand::Prepare {
                buffer_multiplier: requested_multiplier,
                patch,
                known_voicing,
            } => {
                buffer_multiplier = requested_multiplier;
                status.lock().unwrap().phase = KeyboardConnectionPhase::PatchSetting;
                let started = Instant::now();
                let result = supervisor
                    .set_live_buffer_multiplier(u16::from(buffer_multiplier))
                    .and_then(|()| {
                        prepare_patch(
                            supervisor.as_ref(),
                            PatchRequest {
                                patch: patch.as_deref(),
                                known_voicing,
                            },
                        )
                    });
                set_prepare_result(
                    &status,
                    buffer_multiplier,
                    result,
                    PatchRequest {
                        patch: patch.as_deref(),
                        known_voicing,
                    },
                    Some(started.elapsed()),
                );
            }
            KeyboardMidiCommand::SetBufferMultiplier(multiplier) => {
                buffer_multiplier = multiplier;
                let started = Instant::now();
                let result = supervisor.set_live_buffer_multiplier(u16::from(multiplier));
                set_result(
                    &status,
                    buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    false,
                );
            }
            KeyboardMidiCommand::SetPatch {
                note_offs,
                patch,
                known_voicing,
            } => {
                let started = Instant::now();
                let note_off_result = if note_offs.is_empty() {
                    Ok(())
                } else {
                    supervisor
                        .send_midi(KEYBOARD_INSTANCE, &note_offs)
                        .map(|_| ())
                };
                let request = PatchRequest {
                    patch: patch.as_deref(),
                    known_voicing,
                };
                let result =
                    note_off_result.and_then(|()| prepare_patch(supervisor.as_ref(), request));
                set_prepare_result(
                    &status,
                    buffer_multiplier,
                    result,
                    request,
                    Some(started.elapsed()),
                );
            }
            KeyboardMidiCommand::Shutdown => break,
        }
    }
}

fn prepare_patch(
    supervisor: &RealtimePlayServerSupervisor,
    request: PatchRequest<'_>,
) -> Result<Option<VoicingReport>> {
    if request.known_voicing.is_some() {
        supervisor
            .prepare_live_patch(KEYBOARD_INSTANCE, request.patch)
            .map(|()| None)
    } else {
        supervisor.prepare_live_patch_with_voicing(KEYBOARD_INSTANCE, request.patch)
    }
}

#[cfg(test)]
mod tests;
