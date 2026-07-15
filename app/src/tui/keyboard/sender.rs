use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::{
    fast_midi_ipc::FastMidiClient, history::KeyboardTransport,
    realtime_play::RealtimePlayServerSupervisor,
};

mod connection;

use connection::{connect_fast_client, set_result};

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
pub(in crate::tui) struct KeyboardConnectionStatus {
    pub(in crate::tui) transport: KeyboardTransport,
    pub(in crate::tui) phase: KeyboardConnectionPhase,
    pub(in crate::tui) last_send: Option<Duration>,
    pub(in crate::tui) buffer_multiplier: u8,
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
        }
    }

    fn begin_connecting(&mut self, transport: KeyboardTransport, buffer_multiplier: u8) {
        self.transport = transport;
        self.phase = KeyboardConnectionPhase::Connecting;
        self.last_send = None;
        self.buffer_multiplier = buffer_multiplier;
    }
}

enum KeyboardMidiCommand {
    Send {
        messages: Vec<[u8; 3]>,
        patch: Option<String>,
    },
    Stop,
    Prepare {
        transport: KeyboardTransport,
        buffer_multiplier: u8,
        patch: Option<String>,
    },
    SetBufferMultiplier(u8),
    Switch {
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
        self.status
            .lock()
            .unwrap()
            .begin_connecting(transport, buffer_multiplier);
        let _ = self.tx.send(KeyboardMidiCommand::Prepare {
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
        let _ = self.tx.send(KeyboardMidiCommand::Switch {
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

struct WorkerState {
    transport: KeyboardTransport,
    fast_client: Option<FastMidiClient>,
    buffer_multiplier: u8,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self::new(KeyboardTransport::SharedMemory, 4)
    }
}

impl WorkerState {
    fn new(transport: KeyboardTransport, buffer_multiplier: u8) -> Self {
        Self {
            transport,
            fast_client: None,
            buffer_multiplier,
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
                transport,
                buffer_multiplier,
                patch,
            } => {
                let _ = stop(&mut worker, supervisor.as_ref());
                worker.fast_client = None;
                worker.transport = transport;
                worker.buffer_multiplier = buffer_multiplier;
                let result =
                    prepare_connection(&mut worker, supervisor.as_ref(), &status, patch.as_deref());
                set_result(
                    &status,
                    worker.transport,
                    worker.buffer_multiplier,
                    result,
                    None,
                    false,
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
            KeyboardMidiCommand::Switch {
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
                );
            }
            KeyboardMidiCommand::Shutdown => break,
        }
    }
}

fn switch_transport(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<KeyboardConnectionStatus>,
    transport: KeyboardTransport,
    note_offs: &[[u8; 3]],
    patch: Option<&str>,
) {
    let started = Instant::now();
    let old_result = if note_offs.is_empty() {
        Ok(())
    } else {
        send_midi(worker, supervisor, note_offs, patch)
    }
    .and_then(|()| stop(worker, supervisor));

    worker.transport = transport;
    worker.fast_client = None;
    let connect_result = prepare_connection(worker, supervisor, status, patch);
    let result = old_result.and(connect_result);
    set_result(
        status,
        worker.transport,
        worker.buffer_multiplier,
        result,
        Some(started.elapsed()),
        false,
    );
}

fn prepare_connection(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<KeyboardConnectionStatus>,
    patch: Option<&str>,
) -> Result<()> {
    match worker.transport {
        KeyboardTransport::Http => supervisor.set_live_buffer_multiplier(worker.buffer_multiplier),
        KeyboardTransport::SharedMemory => ensure_fast_client(worker, supervisor),
    }?;
    status.lock().unwrap().phase = KeyboardConnectionPhase::PatchSetting;
    supervisor.prepare_live_patch(patch)
}

fn send_midi(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
    messages: &[[u8; 3]],
    patch: Option<&str>,
) -> Result<()> {
    match worker.transport {
        KeyboardTransport::Http => supervisor.send_midi(messages, patch),
        KeyboardTransport::SharedMemory => {
            ensure_fast_client(worker, supervisor)?;
            let result = worker
                .fast_client
                .as_mut()
                .expect("fast client was initialized")
                .send_midi(messages, patch);
            if result.is_err() {
                worker.fast_client = None;
            }
            result
        }
    }
}

fn stop(worker: &mut WorkerState, supervisor: &RealtimePlayServerSupervisor) -> Result<()> {
    match worker.transport {
        KeyboardTransport::Http => supervisor.stop(),
        KeyboardTransport::SharedMemory => match worker.fast_client.as_mut() {
            Some(client) => client.stop(),
            None => Ok(()),
        },
    }
}

fn set_buffer_multiplier(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
) -> Result<()> {
    match worker.transport {
        KeyboardTransport::Http => supervisor.set_live_buffer_multiplier(worker.buffer_multiplier),
        KeyboardTransport::SharedMemory => {
            supervisor.remember_live_buffer_multiplier(worker.buffer_multiplier)?;
            if worker.fast_client.is_none() {
                return ensure_fast_client(worker, supervisor);
            }
            ensure_fast_client(worker, supervisor)?;
            let result = worker
                .fast_client
                .as_mut()
                .expect("fast client was initialized")
                .set_buffer_multiplier(worker.buffer_multiplier);
            if result.is_err() {
                worker.fast_client = None;
            }
            result
        }
    }
}

fn ensure_fast_client(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
) -> Result<()> {
    if worker.fast_client.is_none() {
        supervisor.ensure_started_for_fast_midi()?;
        let mut client = connect_fast_client(supervisor.port())?;
        client.set_buffer_multiplier(worker.buffer_multiplier)?;
        worker.fast_client = Some(client);
    }
    Ok(())
}

#[cfg(test)]
#[path = "sender_tests.rs"]
mod tests;
