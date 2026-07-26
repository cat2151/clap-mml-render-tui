use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::{
    fast_midi_ipc::{FastIpcError, FastMidiClient, FastMidiEvent, InstanceId, LimiterMeter},
    logging::log_realtime_play_event,
    RealtimePlayServerSupervisor,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchVoicing {
    Mono,
    Poly,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ProbeReport {
    pub result: PatchVoicing,
    pub ended_note_ids: Vec<u32>,
    pub blocks: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct VoiceInfoReport {
    pub voice_count: u32,
    pub voice_capacity: u32,
    pub supports_overlapping_notes: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SurgeParamsReport {
    pub scene_mode: String,
    pub active_scene: String,
    pub scene_a_play_mode: String,
    pub scene_b_play_mode: String,
    pub result: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct VoicingReport {
    pub decision: PatchVoicing,
    pub probe: ProbeReport,
    pub voice_info: Option<VoiceInfoReport>,
    pub surge: Option<SurgeParamsReport>,
    pub disagreement: bool,
}

impl RealtimePlayServerSupervisor {
    pub fn send_live_events(&self, events: &[FastMidiEvent]) -> Result<LimiterMeter> {
        self.with_fast_client(|client| {
            client.send_events(events)?;
            Ok(client.limiter_meter())
        })
    }

    pub fn send_midi(&self, instance_id: InstanceId, messages: &[[u8; 3]]) -> Result<LimiterMeter> {
        let events = messages
            .iter()
            .map(|message| FastMidiEvent {
                instance_id,
                offset_frames: 0,
                message: *message,
            })
            .collect::<Vec<_>>();
        self.send_live_events(&events)
    }

    pub fn prepare_live_patch(&self, instance_id: InstanceId, patch: Option<&str>) -> Result<()> {
        let started = Instant::now();
        log_realtime_play_event(format!(
            "action=shm-patch-prepare event=start instance={instance_id} patch={patch:?}"
        ));
        let result = self.with_fast_client(|client| client.prepare_patch(instance_id, patch));
        let elapsed_ms = started.elapsed().as_millis();
        match &result {
            Ok(()) => log_realtime_play_event(format!(
                "action=shm-patch-prepare event=success instance={instance_id} \
                 elapsed_ms={elapsed_ms} patch={patch:?}"
            )),
            Err(error) => log_realtime_play_event(format!(
                "action=shm-patch-prepare event=error instance={instance_id} \
                 elapsed_ms={elapsed_ms} patch={patch:?} error=\"{}\"",
                super::logging::truncate_for_log(&format!("{error:#}"), 1_000)
            )),
        }
        result
    }

    pub fn prepare_live_patch_with_voicing(
        &self,
        instance_id: InstanceId,
        patch: Option<&str>,
    ) -> Result<Option<VoicingReport>> {
        self.prepare_live_patch_with_voicing_traced(instance_id, patch, None)
    }

    pub fn prepare_live_patch_with_voicing_traced(
        &self,
        instance_id: InstanceId,
        patch: Option<&str>,
        probe_id: Option<u64>,
    ) -> Result<Option<VoicingReport>> {
        let started = Instant::now();
        let bytes = self.with_fast_client(|client| client.probe_patch(instance_id, patch))?;
        let report = serde_json::from_slice(&bytes)?;
        log_realtime_play_event(format!(
            "action=shm-patch-probe probe_id={} instance={} elapsed_ms={} patch={patch:?}",
            probe_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "untracked".to_string()),
            instance_id,
            started.elapsed().as_millis()
        ));
        Ok(Some(report))
    }

    pub fn stop_live_instance(&self, instance_id: InstanceId) -> Result<()> {
        self.with_fast_client(|client| client.stop(instance_id))
    }

    pub fn stop_live_all(&self) -> Result<()> {
        let mut client = self.fast_client.lock().unwrap();
        match client.as_mut() {
            Some(client) => client.stop_all().map_err(Into::into),
            None => Ok(()),
        }
    }

    pub fn set_live_buffer_multiplier(&self, multiplier: u8) -> Result<()> {
        validate_buffer_multiplier(multiplier)?;
        *self.live_buffer_multiplier.lock().unwrap() = multiplier;
        self.with_fast_client(|client| client.set_buffer_multiplier(multiplier))
    }

    pub fn set_connected_live_buffer_multiplier(&self, multiplier: u8) -> Result<()> {
        validate_buffer_multiplier(multiplier)?;
        *self.live_buffer_multiplier.lock().unwrap() = multiplier;
        let mut client = self.fast_client.lock().unwrap();
        match client.as_mut() {
            Some(client) => client.set_buffer_multiplier(multiplier).map_err(Into::into),
            None => Ok(()),
        }
    }

    pub fn remember_live_buffer_multiplier(&self, multiplier: u8) -> Result<()> {
        validate_buffer_multiplier(multiplier)?;
        *self.live_buffer_multiplier.lock().unwrap() = multiplier;
        Ok(())
    }

    pub fn limiter_meter(&self) -> LimiterMeter {
        self.fast_client
            .lock()
            .unwrap()
            .as_ref()
            .map(FastMidiClient::limiter_meter)
            .unwrap_or_default()
    }

    pub fn underrun_frames(&self) -> u64 {
        self.fast_client
            .lock()
            .unwrap()
            .as_ref()
            .map(FastMidiClient::underrun_frames)
            .unwrap_or(0)
    }

    fn with_fast_client<T>(
        &self,
        operation: impl FnOnce(&mut FastMidiClient) -> Result<T, FastIpcError>,
    ) -> Result<T> {
        self.ensure_started_for_fast_midi()?;
        let mut client = self.fast_client.lock().unwrap();
        if client.is_none() {
            let mut connected = connect_fast_client(self.port)?;
            connected.set_buffer_multiplier(*self.live_buffer_multiplier.lock().unwrap())?;
            *client = Some(connected);
        }
        let result = operation(client.as_mut().expect("client was initialized"));
        if matches!(
            result,
            Err(FastIpcError::ServerStopped | FastIpcError::ProtocolMismatch)
        ) {
            *client = None;
        }
        result.map_err(|error| anyhow!(error))
    }
}

fn validate_buffer_multiplier(multiplier: u8) -> Result<()> {
    if !matches!(multiplier, 1 | 2 | 4 | 8 | 16) {
        anyhow::bail!("buffer multiplier must be 1, 2, 4, 8, or 16");
    }
    Ok(())
}

#[cfg(windows)]
fn connect_fast_client(port: u16) -> Result<FastMidiClient, FastIpcError> {
    use std::time::Duration;

    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        match FastMidiClient::connect(port) {
            Ok(client) => return Ok(client),
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(not(windows))]
fn connect_fast_client(port: u16) -> Result<FastMidiClient, FastIpcError> {
    FastMidiClient::connect(port)
}
