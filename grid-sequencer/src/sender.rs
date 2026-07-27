use std::{
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use cmrt_realtime_play::{FastMidiEvent, LimiterMeter, RealtimePlayServerSupervisor};

mod adaptive_buffer;
mod status;

use adaptive_buffer::{AdaptiveBuffer, INITIAL_BUFFER_MULTIPLIER, RESTORE_BUFFER_MULTIPLIER};
pub use status::{GridConnectionPhase, GridConnectionStatus, GridProgress, GridRowReadiness};

const METER_POLL_INTERVAL: Duration = Duration::from_millis(50);

enum GridMidiCommand {
    StartServer,
    Send { events: Vec<FastMidiEvent> },
    Prepare { patches: Vec<Option<String>> },
    Stop,
    Shutdown,
}

pub struct GridMidiSender {
    tx: mpsc::Sender<GridMidiCommand>,
    status: Arc<Mutex<GridConnectionStatus>>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    worker: Option<JoinHandle<()>>,
}

impl GridMidiSender {
    pub fn new(supervisor: Arc<RealtimePlayServerSupervisor>) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(GridConnectionStatus::default()));
        let worker_status = Arc::clone(&status);
        let worker_supervisor = Arc::clone(&supervisor);
        let worker = std::thread::Builder::new()
            .name("grid-sequencer-midi-sender".to_string())
            .spawn(move || run_midi_sender(rx, worker_supervisor, worker_status))
            .expect("grid sequencer MIDI sender thread should start");
        Self {
            tx,
            status,
            supervisor,
            worker: Some(worker),
        }
    }

    pub fn start_server(&self) {
        self.status.lock().unwrap().begin_connecting();
        let _ = self.tx.send(GridMidiCommand::StartServer);
    }

    pub fn send_scheduled(&self, events: Vec<FastMidiEvent>) {
        if !events.is_empty() {
            let _ = self.tx.send(GridMidiCommand::Send { events });
        }
    }

    pub fn prepare<'a>(&self, patches: impl Iterator<Item = Option<&'a str>>) {
        self.status.lock().unwrap().begin_connecting();
        let patches = patches
            .map(|patch| patch.map(str::to_string))
            .collect::<Vec<_>>();
        let _ = self.tx.send(GridMidiCommand::Prepare { patches });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(GridMidiCommand::Stop);
    }

    pub fn status(&self) -> GridConnectionStatus {
        let mut status = self.status.lock().unwrap().clone();
        if matches!(status.phase, GridConnectionPhase::Connecting) {
            if let Some(progress) = self.supervisor.startup_progress() {
                status.update_server_startup(
                    progress.initialized_instances,
                    progress.total_instances,
                );
            }
        }
        status
    }
}

impl Drop for GridMidiSender {
    fn drop(&mut self) {
        let _ = self.tx.send(GridMidiCommand::Stop);
        let _ = self.tx.send(GridMidiCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_midi_sender(
    rx: mpsc::Receiver<GridMidiCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<GridConnectionStatus>>,
) {
    let mut adaptive_buffer = None;
    loop {
        let command = match rx.recv_timeout(METER_POLL_INTERVAL) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                poll_runtime_status(
                    supervisor.as_ref(),
                    &status,
                    &mut adaptive_buffer,
                    Instant::now(),
                );
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match command {
            GridMidiCommand::StartServer => {
                adaptive_buffer = None;
                let started = Instant::now();
                match supervisor.ensure_started_for_fast_midi() {
                    Ok(()) => status.lock().unwrap().wait_for_patches(started.elapsed()),
                    Err(error) => apply(&status, Err(error), Some(started.elapsed()), false),
                }
            }
            GridMidiCommand::Send { events } => {
                let started = Instant::now();
                let result = supervisor.send_live_events(&events);
                apply(&status, result, Some(started.elapsed()), false);
            }
            GridMidiCommand::Prepare { patches } => {
                adaptive_buffer = None;
                let started = Instant::now();
                let result = supervisor.ensure_started_for_fast_midi().and_then(|()| {
                    status.lock().unwrap().begin_patch_setting(patches.len());
                    prepare_instances(supervisor.as_ref(), &patches, |completed, total| {
                        status
                            .lock()
                            .unwrap()
                            .update_patch_setting(completed, total);
                    })
                });
                if result.is_ok() {
                    let now = Instant::now();
                    let buffer = AdaptiveBuffer::new(now, supervisor.underrun_frames());
                    status
                        .lock()
                        .unwrap()
                        .update_adaptive_buffer(buffer.multiplier(), buffer.underrun_frames());
                    adaptive_buffer = Some(buffer);
                }
                apply(
                    &status,
                    result.map(|()| supervisor.limiter_meter()),
                    Some(started.elapsed()),
                    false,
                );
            }
            GridMidiCommand::Stop => {
                adaptive_buffer = None;
                let started = Instant::now();
                let result = supervisor
                    .stop_live_all()
                    .and_then(|()| {
                        supervisor.set_connected_live_buffer_multiplier(RESTORE_BUFFER_MULTIPLIER)
                    })
                    .map(|()| LimiterMeter::default());
                status
                    .lock()
                    .unwrap()
                    .update_adaptive_buffer(RESTORE_BUFFER_MULTIPLIER, 0);
                apply(&status, result, Some(started.elapsed()), true);
            }
            GridMidiCommand::Shutdown => break,
        }
    }
}

fn prepare_instances(
    supervisor: &RealtimePlayServerSupervisor,
    patches: &[Option<String>],
    mut report_progress: impl FnMut(usize, usize),
) -> anyhow::Result<()> {
    if patches.len() != cmrt_realtime_play::INSTANCE_COUNT {
        anyhow::bail!(
            "grid requires {} patches, got {}",
            cmrt_realtime_play::INSTANCE_COUNT,
            patches.len()
        );
    }
    supervisor.stop_live_all()?;
    supervisor.set_live_buffer_multiplier(INITIAL_BUFFER_MULTIPLIER)?;
    report_progress(0, patches.len());
    for (instance_id, patch) in patches.iter().enumerate() {
        if let Err(error) = supervisor
            .prepare_live_patch(instance_id as u8, patch.as_deref())
            .with_context(|| {
                format!(
                    "grid row {instance_id} patch prepare failed (patch={:?})",
                    patch.as_deref()
                )
            })
        {
            let _ = supervisor.stop_live_all();
            return Err(error);
        }
        report_progress(instance_id + 1, patches.len());
    }
    Ok(())
}

fn poll_runtime_status(
    supervisor: &RealtimePlayServerSupervisor,
    status: &Mutex<GridConnectionStatus>,
    adaptive_buffer: &mut Option<AdaptiveBuffer>,
    now: Instant,
) {
    status
        .lock()
        .unwrap()
        .update_limiter_meter(supervisor.limiter_meter());
    if !status.lock().unwrap().phase.accepts_notes() {
        return;
    }
    let Some(buffer) = adaptive_buffer.as_mut() else {
        return;
    };
    let adjustment = buffer.observe(now, supervisor.underrun_frames());
    if let Some(multiplier) = adjustment {
        let previous = status.lock().unwrap().buffer_multiplier;
        if let Err(error) = supervisor.set_live_buffer_multiplier(multiplier) {
            apply(status, Err(error), None, false);
            return;
        }
        let reason = if multiplier > previous {
            "underrun"
        } else {
            "stable"
        };
        crate::log_line(&format!(
            "grid-sequencer: buffer auto {previous} -> {multiplier} reason={reason}"
        ));
    }
    status
        .lock()
        .unwrap()
        .update_adaptive_buffer(buffer.multiplier(), buffer.underrun_frames());
}

fn apply(
    status: &Mutex<GridConnectionStatus>,
    result: anyhow::Result<LimiterMeter>,
    elapsed: Option<std::time::Duration>,
    idle_on_success: bool,
) {
    if let Err(error) = &result {
        crate::log_line(&format!("grid-sequencer: MIDI worker error: {error:#}"));
    }
    status
        .lock()
        .unwrap()
        .apply_result(result, elapsed, idle_on_success);
}

#[cfg(test)]
mod tests;
