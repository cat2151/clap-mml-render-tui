use std::{io::Read as _, time::Instant};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::{PlayRequestError, RealtimePlayServerSupervisor, PLAY_CONTENT_TYPE_JSON};

const PLAY_SERVER_LIVE_PATCH_PATH: &str = "/live-patch";
const PLAY_SERVER_LIVE_PATCH_PROBE_PATH: &str = "/live-patch-probe";

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
    pub fn prepare_live_patch(&self, patch: Option<&str>) -> Result<()> {
        let body = serde_json::to_vec(&serde_json::json!({ "patch": patch }))?;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_post_bytes(
                PLAY_SERVER_LIVE_PATCH_PATH,
                Some((PLAY_CONTENT_TYPE_JSON, &body)),
            ) {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(_)) => {
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    pub fn prepare_live_patch_with_voicing(
        &self,
        patch: Option<&str>,
    ) -> Result<Option<VoicingReport>> {
        self.prepare_live_patch_with_voicing_inner(patch, None)
    }

    pub fn prepare_live_patch_with_voicing_traced(
        &self,
        patch: Option<&str>,
        probe_id: u64,
    ) -> Result<Option<VoicingReport>> {
        self.prepare_live_patch_with_voicing_inner(patch, Some(probe_id))
    }

    fn prepare_live_patch_with_voicing_inner(
        &self,
        patch: Option<&str>,
        probe_id: Option<u64>,
    ) -> Result<Option<VoicingReport>> {
        let started = Instant::now();
        let mut retry = 0;
        log_probe_event(
            probe_id,
            format!("event=http-start retry=0 patch={patch:?}"),
        );
        let body = serde_json::to_vec(&serde_json::json!({ "patch": patch }))?;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_live_patch_probe_once(&body) {
                Ok(report) => {
                    log_probe_event(
                        probe_id,
                        format!(
                            "event=http-response elapsed_ms={} patch={patch:?} report={report:?}",
                            started.elapsed().as_millis()
                        ),
                    );
                    return Ok(Some(report));
                }
                Err(PlayRequestError::Server {
                    status: 404 | 405, ..
                }) => {
                    log_probe_event(
                        probe_id,
                        format!(
                            "event=http-fallback elapsed_ms={} patch={patch:?} reason=legacy-server",
                            started.elapsed().as_millis()
                        ),
                    );
                    self.prepare_live_patch(patch)?;
                    return Ok(None);
                }
                Err(PlayRequestError::Server { status, message }) => {
                    log_probe_event(
                        probe_id,
                        format!(
                            "event=http-error elapsed_ms={} patch={patch:?} status={status} error={message:?}",
                            started.elapsed().as_millis()
                        ),
                    );
                    return Err(anyhow!(message));
                }
                Err(PlayRequestError::Transport(message)) => {
                    retry += 1;
                    log_probe_event(
                        probe_id,
                        format!(
                            "event=http-retry elapsed_ms={} retry={retry} patch={patch:?} error={message:?}",
                            started.elapsed().as_millis()
                        ),
                    );
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    fn send_live_patch_probe_once(
        &self,
        body: &[u8],
    ) -> std::result::Result<VoicingReport, PlayRequestError> {
        let url = format!(
            "http://127.0.0.1:{}{}",
            self.port, PLAY_SERVER_LIVE_PATCH_PROBE_PATH
        );
        match self
            .agent
            .post(&url)
            .set("Content-Type", PLAY_CONTENT_TYPE_JSON)
            .send_bytes(body)
        {
            Ok(response) => {
                let status = response.status();
                let mut response_body = String::new();
                response
                    .into_reader()
                    .read_to_string(&mut response_body)
                    .map_err(|error| PlayRequestError::Server {
                        status,
                        message: format!("voicing report could not be read: {error}"),
                    })?;
                serde_json::from_str(&response_body).map_err(|error| PlayRequestError::Server {
                    status,
                    message: format!("voicing report is invalid: {error}"),
                })
            }
            Err(ureq::Error::Status(status, response)) => {
                let body = super::response_body(response);
                let message = if body.trim().is_empty() {
                    format!("realtime play server returned HTTP {status}")
                } else {
                    format!(
                        "realtime play server returned HTTP {status}: {}",
                        body.trim()
                    )
                };
                Err(PlayRequestError::Server { status, message })
            }
            Err(ureq::Error::Transport(error)) => {
                Err(PlayRequestError::Transport(error.to_string()))
            }
        }
    }
}

fn log_probe_event(probe_id: Option<u64>, message: String) {
    let probe_id = probe_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "untracked".to_string());
    super::log_realtime_play_event(format!(
        "action=live-patch-probe probe_id={probe_id} {message}"
    ));
}
