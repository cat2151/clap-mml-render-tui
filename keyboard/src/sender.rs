use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

use cmrt_realtime_play::{
    fast_midi_ipc::FastMidiClient, PatchVoicing, RealtimePlayServerSupervisor,
};

use crate::session_state::KeyboardTransport;

mod connection;
mod status;
mod voicing_trace;
mod worker;

use connection::{connect_fast_client, set_prepare_result, set_result};
pub use status::{KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardVoicingStatus};
use voicing_trace::VoicingTrace;
use worker::{
    prepare_connection, send_midi, set_buffer_multiplier, stop, switch_transport, WorkerState,
};

/// 適用する patch と、その patch について file cache から分かっている判定結果。
/// cache ヒット時は probe を省くので、この 2 つは常に組で受け渡す。
#[derive(Clone, Copy, Debug)]
pub(super) struct PatchRequest<'a> {
    pub(super) patch: Option<&'a str>,
    pub(super) known_voicing: Option<PatchVoicing>,
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
        known_voicing: Option<PatchVoicing>,
    },
    SetBufferMultiplier(u8),
    SetPatch {
        trace: VoicingTrace,
        note_offs: Vec<[u8; 3]>,
        previous_patch: Option<String>,
        patch: Option<String>,
        known_voicing: Option<PatchVoicing>,
    },
    Switch {
        trace: VoicingTrace,
        transport: KeyboardTransport,
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
    pub fn new(
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

    pub fn send(&self, messages: Vec<[u8; 3]>, patch: Option<&str>) {
        let _ = self.tx.send(KeyboardMidiCommand::Send {
            messages,
            patch: patch.map(str::to_string),
        });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(KeyboardMidiCommand::Stop);
    }

    pub fn prepare(
        &self,
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        let trace = VoicingTrace::queued("prepare", transport, buffer_multiplier, None, patch);
        self.status.lock().unwrap().begin_connecting(
            transport,
            buffer_multiplier,
            patch,
            known_voicing,
        );
        let _ = self.tx.send(KeyboardMidiCommand::Prepare {
            trace,
            transport,
            buffer_multiplier,
            patch: patch.map(str::to_string),
            known_voicing,
        });
    }

    pub fn switch(
        &self,
        transport: KeyboardTransport,
        note_offs: Vec<[u8; 3]>,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
    ) {
        let mut status = self.status.lock().unwrap();
        let buffer_multiplier = status.buffer_multiplier;
        status.begin_connecting(transport, buffer_multiplier, patch, known_voicing);
        drop(status);
        let trace = VoicingTrace::queued("switch", transport, buffer_multiplier, patch, patch);
        let _ = self.tx.send(KeyboardMidiCommand::Switch {
            trace,
            transport,
            note_offs,
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
        previous_patch: Option<&str>,
        patch: Option<&str>,
        known_voicing: Option<PatchVoicing>,
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
        self.status
            .lock()
            .unwrap()
            .begin_patch_setting(patch, known_voicing);
        let _ = self.tx.send(KeyboardMidiCommand::SetPatch {
            trace,
            note_offs,
            previous_patch: previous_patch.map(str::to_string),
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
                known_voicing,
            } => {
                trace.worker_started(transport, buffer_multiplier, patch.as_deref());
                let started = Instant::now();
                let _ = stop(&mut worker, supervisor.as_ref());
                worker.fast_client = None;
                worker.transport = transport;
                worker.buffer_multiplier = buffer_multiplier;
                let request = PatchRequest {
                    patch: patch.as_deref(),
                    known_voicing,
                };
                let result = prepare_connection(
                    &mut worker,
                    supervisor.as_ref(),
                    &status,
                    request,
                    trace.id(),
                );
                trace.status_apply(
                    worker.transport,
                    patch.as_deref(),
                    &result,
                    known_voicing,
                    started.elapsed().as_millis(),
                );
                set_prepare_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    request,
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
                known_voicing,
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
                let request = PatchRequest {
                    patch: patch.as_deref(),
                    known_voicing,
                };
                let patch_result = prepare_connection(
                    &mut worker,
                    supervisor.as_ref(),
                    &status,
                    request,
                    trace.id(),
                );
                let result = note_off_result.and(patch_result);
                trace.status_apply(
                    worker.transport,
                    patch.as_deref(),
                    &result,
                    known_voicing,
                    started.elapsed().as_millis(),
                );
                set_prepare_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    request,
                    Some(started.elapsed()),
                );
            }
            KeyboardMidiCommand::Switch {
                trace,
                transport,
                note_offs,
                patch,
                known_voicing,
            } => {
                switch_transport(
                    &mut worker,
                    supervisor.as_ref(),
                    &status,
                    transport,
                    &note_offs,
                    PatchRequest {
                        patch: patch.as_deref(),
                        known_voicing,
                    },
                    trace,
                );
            }
            KeyboardMidiCommand::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests;
