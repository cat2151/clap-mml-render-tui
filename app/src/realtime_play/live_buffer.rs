use anyhow::{anyhow, Result};

use super::{PlayRequestError, RealtimePlayServerSupervisor, PLAY_CONTENT_TYPE_JSON};

const PLAY_SERVER_LIVE_BUFFER_PATH: &str = "/live-buffer";

pub(super) struct LiveBufferState {
    pub(super) desired_multiplier: u8,
    pub(super) applied: Option<(u64, u8)>,
}

impl Default for LiveBufferState {
    fn default() -> Self {
        Self {
            desired_multiplier: 4,
            applied: None,
        }
    }
}

impl RealtimePlayServerSupervisor {
    pub(crate) fn remember_live_buffer_multiplier(&self, multiplier: u8) -> Result<()> {
        if !matches!(multiplier, 1 | 2 | 4 | 8) {
            anyhow::bail!("live buffer multiplier must be 1, 2, 4, or 8");
        }
        let mut state = self.live_buffer.lock().unwrap();
        state.desired_multiplier = multiplier;
        state.applied = None;
        Ok(())
    }

    pub(crate) fn set_live_buffer_multiplier(&self, multiplier: u8) -> Result<()> {
        self.remember_live_buffer_multiplier(multiplier)?;
        loop {
            let server_generation = self.ensure_started()?;
            match self.apply_live_buffer_once(server_generation) {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(_)) => {
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    pub(super) fn apply_live_buffer_once(
        &self,
        server_generation: u64,
    ) -> std::result::Result<(), PlayRequestError> {
        let multiplier = {
            let state = self.live_buffer.lock().unwrap();
            if state.applied == Some((server_generation, state.desired_multiplier)) {
                return Ok(());
            }
            state.desired_multiplier
        };
        let body = serde_json::to_vec(&serde_json::json!({ "multiplier": multiplier }))
            .expect("serializing a numeric live buffer request cannot fail");
        self.send_post_bytes(
            PLAY_SERVER_LIVE_BUFFER_PATH,
            Some((PLAY_CONTENT_TYPE_JSON, &body)),
        )?;
        let mut state = self.live_buffer.lock().unwrap();
        if state.desired_multiplier == multiplier {
            state.applied = Some((server_generation, multiplier));
        }
        Ok(())
    }
}
