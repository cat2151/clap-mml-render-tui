use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    fast_midi_ipc::FastMidiClient,
    history::KeyboardTransport,
    realtime_play::{PatchVoicing, RealtimePlayServerSupervisor, VoicingReport},
};

mod connection;
mod voicing_trace;
mod worker;

use connection::{connect_fast_client, set_prepare_result, set_result};
use voicing_trace::VoicingTrace;
use worker::{
    prepare_connection, send_midi, set_buffer_multiplier, stop, switch_transport, WorkerState,
};

impl KeyboardTransport {
    pub(in crate::tui) fn label(self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::SharedMemory => "SHM",
        }
    }

    pub(in crate::tui) fn toggled(self) -> Self {
        match self {
            Self::Http => Self::SharedMemory,
            Self::SharedMemory => Self::Http,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) enum KeyboardConnectionPhase {
    Idle,
    Connecting,
    PatchSetting,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) enum KeyboardVoicingStatus {
    Unavailable,
    Detecting { previous: Option<VoicingReport> },
    Detected(VoicingReport),
}

impl KeyboardVoicingStatus {
    pub(super) fn effective_decision(&self) -> PatchVoicing {
        match self {
            Self::Detected(report)
            | Self::Detecting {
                previous: Some(report),
            } => report.decision,
            Self::Unavailable | Self::Detecting { previous: None } => PatchVoicing::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) struct KeyboardConnectionStatus {
    pub(in crate::tui) transport: KeyboardTransport,
    pub(in crate::tui) phase: KeyboardConnectionPhase,
    pub(in crate::tui) last_send: Option<Duration>,
    pub(in crate::tui) buffer_multiplier: u8,
    pub(in crate::tui) voicing: KeyboardVoicingStatus,
}

impl Default for KeyboardConnectionStatus {
    fn default() -> Self {
        Self::new(KeyboardTransport::SharedMemory, 4)
    }
}

impl KeyboardConnectionStatus {
    fn new(transport: KeyboardTransport, buffer_multiplier: u8) -> Self {
        Self {
            transport,
            phase: KeyboardConnectionPhase::Idle,
            last_send: None,
            buffer_multiplier,
            voicing: KeyboardVoicingStatus::Unavailable,
        }
    }

    fn begin_connecting(&mut self, transport: KeyboardTransport, buffer_multiplier: u8) {
        self.transport = transport;
        self.phase = KeyboardConnectionPhase::Connecting;
        self.last_send = None;
        self.buffer_multiplier = buffer_multiplier;
        self.begin_voicing_probe();
    }

    fn begin_patch_setting(&mut self) {
        self.phase = KeyboardConnectionPhase::PatchSetting;
        self.begin_voicing_probe();
    }

    fn begin_voicing_probe(&mut self) {
        let previous =
            match std::mem::replace(&mut self.voicing, KeyboardVoicingStatus::Unavailable) {
                KeyboardVoicingStatus::Detected(report) => Some(report),
                KeyboardVoicingStatus::Detecting { previous } => previous,
                KeyboardVoicingStatus::Unavailable => None,
            };
        self.voicing = KeyboardVoicingStatus::Detecting { previous };
    }
}

enum KeyboardMidiCommand {
    Send {
        messages: Vec<[u8; 3]>,
        patch: Option<String>,
    },
    Stop,
    Prepare {
        trace: VoicingTrace,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        patch: Option<String>,
    },
    SetBufferMultiplier(u8),
    SetPatch {
        trace: VoicingTrace,
        note_offs: Vec<[u8; 3]>,
        previous_patch: Option<String>,
        patch: Option<String>,
    },
    Switch {
        trace: VoicingTrace,
        transport: KeyboardTransport,
        note_offs: Vec<[u8; 3]>,
        patch: Option<String>,
    },
    Shutdown,
}

pub(in crate::tui) struct KeyboardMidiSender {
    tx: mpsc::Sender<KeyboardMidiCommand>,
    status: Arc<Mutex<KeyboardConnectionStatus>>,
    worker: Option<JoinHandle<()>>,
}

impl KeyboardMidiSender {
    pub(in crate::tui) fn new(
        supervisor: Arc<RealtimePlayServerSupervisor>,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(KeyboardConnectionStatus::new(
            transport,
            buffer_multiplier,
        )));
        let worker_status = Arc::clone(&status);
        let worker = std::thread::Builder::new()
            .name("keyboard-midi-sender".to_string())
            .spawn(move || {
                run_midi_sender(rx, supervisor, worker_status, transport, buffer_multiplier)
            })
            .expect("keyboard MIDI sender thread should start");
        Self {
            tx,
            status,
            worker: Some(worker),
        }
    }

    pub(in crate::tui) fn send(&self, messages: Vec<[u8; 3]>, patch: Option<&str>) {
        let _ = self.tx.send(KeyboardMidiCommand::Send {
            messages,
            patch: patch.map(str::to_string),
        });
    }

    pub(in crate::tui) fn stop(&self) {
        let _ = self.tx.send(KeyboardMidiCommand::Stop);
    }

    pub(in crate::tui) fn prepare(
        &self,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        patch: Option<&str>,
    ) {
        let trace = VoicingTrace::queued("prepare", transport, buffer_multiplier, None, patch);
        self.status
            .lock()
            .unwrap()
            .begin_connecting(transport, buffer_multiplier);
        let _ = self.tx.send(KeyboardMidiCommand::Prepare {
            trace,
            transport,
            buffer_multiplier,
            patch: patch.map(str::to_string),
        });
    }

    pub(in crate::tui) fn switch(
        &self,
        transport: KeyboardTransport,
        note_offs: Vec<[u8; 3]>,
        patch: Option<&str>,
    ) {
        let mut status = self.status.lock().unwrap();
        let buffer_multiplier = status.buffer_multiplier;
        status.begin_connecting(transport, buffer_multiplier);
        drop(status);
        let trace = VoicingTrace::queued("switch", transport, buffer_multiplier, patch, patch);
        let _ = self.tx.send(KeyboardMidiCommand::Switch {
            trace,
            transport,
            note_offs,
            patch: patch.map(str::to_string),
        });
    }

    pub(in crate::tui) fn set_buffer_multiplier(&self, multiplier: u8) {
        let _ = self
            .tx
            .send(KeyboardMidiCommand::SetBufferMultiplier(multiplier));
    }

    pub(in crate::tui) fn set_patch(
        &self,
        note_offs: Vec<[u8; 3]>,
        previous_patch: Option<&str>,
        patch: Option<&str>,
    ) {
        let status = self.status.lock().unwrap();
        let transport = status.transport;
        let buffer_multiplier = status.buffer_multiplier;
        drop(status);
        let trace = VoicingTrace::queued(
            "set-patch",
            transport,
            buffer_multiplier,
            previous_patch,
            patch,
        );
        self.status.lock().unwrap().begin_patch_setting();
        let _ = self.tx.send(KeyboardMidiCommand::SetPatch {
            trace,
            note_offs,
            previous_patch: previous_patch.map(str::to_string),
            patch: patch.map(str::to_string),
        });
    }

    pub(in crate::tui) fn status(&self) -> KeyboardConnectionStatus {
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
    transport: KeyboardTransport,
    buffer_multiplier: u8,
) {
    let mut worker = WorkerState::new(transport, buffer_multiplier);
    let _ = supervisor.remember_live_buffer_multiplier(buffer_multiplier);
    while let Ok(command) = rx.recv() {
        match command {
            KeyboardMidiCommand::Send { messages, patch } => {
                let started = Instant::now();
                let result = send_midi(
                    &mut worker,
                    supervisor.as_ref(),
                    &messages,
                    patch.as_deref(),
                );
                set_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    false,
                );
            }
            KeyboardMidiCommand::Stop => {
                let started = Instant::now();
                let result = stop(&mut worker, supervisor.as_ref());
                set_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    true,
                );
            }
            KeyboardMidiCommand::Prepare {
                trace,
                transport,
                buffer_multiplier,
                patch,
            } => {
                trace.worker_started(transport, buffer_multiplier, patch.as_deref());
                let started = Instant::now();
                let _ = stop(&mut worker, supervisor.as_ref());
                worker.fast_client = None;
                worker.transport = transport;
                worker.buffer_multiplier = buffer_multiplier;
                let result = prepare_connection(
                    &mut worker,
                    supervisor.as_ref(),
                    &status,
                    patch.as_deref(),
                    trace.id(),
                );
                trace.status_apply(
                    worker.transport,
                    patch.as_deref(),
                    &result,
                    started.elapsed().as_millis(),
                );
                set_prepare_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    None,
                );
            }
            KeyboardMidiCommand::SetBufferMultiplier(multiplier) => {
                let started = Instant::now();
                worker.buffer_multiplier = multiplier;
                let result = set_buffer_multiplier(&mut worker, supervisor.as_ref());
                set_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                    false,
                );
            }
            KeyboardMidiCommand::SetPatch {
                trace,
                note_offs,
                previous_patch,
                patch,
            } => {
                trace.worker_started(worker.transport, worker.buffer_multiplier, patch.as_deref());
                let started = Instant::now();
                let note_off_result = if note_offs.is_empty() {
                    Ok(())
                } else {
                    send_midi(
                        &mut worker,
                        supervisor.as_ref(),
                        &note_offs,
                        previous_patch.as_deref(),
                    )
                };
                let patch_result = prepare_connection(
                    &mut worker,
                    supervisor.as_ref(),
                    &status,
                    patch.as_deref(),
                    trace.id(),
                );
                let result = note_off_result.and(patch_result);
                trace.status_apply(
                    worker.transport,
                    patch.as_deref(),
                    &result,
                    started.elapsed().as_millis(),
                );
                set_prepare_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    Some(started.elapsed()),
                );
            }
            KeyboardMidiCommand::Switch {
                trace,
                transport,
                note_offs,
                patch,
            } => {
                switch_transport(
                    &mut worker,
                    supervisor.as_ref(),
                    &status,
                    transport,
                    &note_offs,
                    patch.as_deref(),
                    trace,
                );
            }
            KeyboardMidiCommand::Shutdown => break,
        }
    }
}

#[cfg(test)]
#[path = "sender_tests.rs"]
mod tests;
