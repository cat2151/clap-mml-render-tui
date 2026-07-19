use std::{sync::Mutex, time::Instant};

use anyhow::Result;

use super::{
    connect_fast_client, set_prepare_result, voicing_trace::VoicingTrace, FastMidiClient,
    KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardTransport,
    RealtimePlayServerSupervisor,
};
use crate::realtime_play::VoicingReport;

pub(super) struct WorkerState {
    pub(super) transport: KeyboardTransport,
    pub(super) fast_client: Option<FastMidiClient>,
    pub(super) buffer_multiplier: u8,
}

impl WorkerState {
    pub(super) fn new(transport: KeyboardTransport, buffer_multiplier: u8) -> Self {
        Self {
            transport,
            fast_client: None,
            buffer_multiplier,
        }
    }
}

pub(super) fn prepare_connection(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<KeyboardConnectionStatus>,
    patch: Option<&str>,
    probe_id: u64,
) -> Result<Option<VoicingReport>> {
    match worker.transport {
        KeyboardTransport::Http => supervisor.set_live_buffer_multiplier(worker.buffer_multiplier),
        KeyboardTransport::SharedMemory => ensure_fast_client(worker, supervisor),
    }?;
    status.lock().unwrap().phase = KeyboardConnectionPhase::PatchSetting;
    supervisor.prepare_live_patch_with_voicing_traced(patch, probe_id)
}

pub(super) fn send_midi(
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

pub(super) fn stop(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
) -> Result<()> {
    match worker.transport {
        KeyboardTransport::Http => supervisor.stop(),
        KeyboardTransport::SharedMemory => match worker.fast_client.as_mut() {
            Some(client) => client.stop(),
            None => Ok(()),
        },
    }
}

pub(super) fn switch_transport(
    worker: &mut WorkerState,
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<KeyboardConnectionStatus>,
    transport: KeyboardTransport,
    note_offs: &[[u8; 3]],
    patch: Option<&str>,
    trace: VoicingTrace,
) {
    trace.worker_started(transport, worker.buffer_multiplier, patch);
    let started = Instant::now();
    let old_result = if note_offs.is_empty() {
        Ok(())
    } else {
        send_midi(worker, supervisor, note_offs, patch)
    }
    .and_then(|()| stop(worker, supervisor));

    worker.transport = transport;
    worker.fast_client = None;
    let connect_result = prepare_connection(worker, supervisor, status, patch, trace.id());
    let result = old_result.and(connect_result);
    trace.status_apply(
        worker.transport,
        patch,
        &result,
        started.elapsed().as_millis(),
    );
    set_prepare_result(
        status,
        worker.transport,
        worker.buffer_multiplier,
        result,
        Some(started.elapsed()),
    );
}

pub(super) fn set_buffer_multiplier(
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
