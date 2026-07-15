use anyhow::{anyhow, Result};

use super::{PlayRequestError, RealtimePlayServerSupervisor, PLAY_CONTENT_TYPE_JSON};

const PLAY_SERVER_LIVE_PATCH_PATH: &str = "/live-patch";

impl RealtimePlayServerSupervisor {
    pub(crate) fn prepare_live_patch(&self, patch: Option<&str>) -> Result<()> {
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
}
