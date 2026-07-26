use anyhow::Result;
use cmrt_realtime_play::{fast_midi_ipc::FastMidiClient, RealtimePlayServerSupervisor};
use cmrt_tui_core::keyboard_session_state::KeyboardTransport;

use super::BUFFER_MULTIPLIER;

/// ワーカースレッドだけが触る接続リソース。
pub(super) struct WorkerState {
    pub(super) transport: KeyboardTransport,
    fast_client: Option<FastMidiClient>,
}

impl WorkerState {
    pub(super) fn new(transport: KeyboardTransport) -> Self {
        Self {
            transport,
            fast_client: None,
        }
    }

    /// 音色を適用する。live 再生中の `/midi` は patch を無視するサーバー仕様のため、
    /// 音色の切替は必ずこの `/live-patch` 経路を通す必要がある。
    pub(super) fn prepare_patch(
        &mut self,
        supervisor: &RealtimePlayServerSupervisor,
        patch: Option<&str>,
    ) -> Result<()> {
        match self.transport {
            KeyboardTransport::Http => supervisor.set_live_buffer_multiplier(BUFFER_MULTIPLIER),
            KeyboardTransport::SharedMemory => self.ensure_fast_client(supervisor),
        }?;
        supervisor.prepare_live_patch(patch)
    }

    /// `(offset_frames, message)` の並びを送る。offset はサーバーの現在の live 位置から
    /// のフレーム数で、サーバー側でサンプル精度のスケジュールに載る。
    pub(super) fn send_midi(
        &mut self,
        supervisor: &RealtimePlayServerSupervisor,
        events: &[(u32, [u8; 3])],
        patch: Option<&str>,
    ) -> Result<()> {
        match self.transport {
            KeyboardTransport::Http => supervisor.send_midi_with_offsets(events, patch),
            KeyboardTransport::SharedMemory => {
                self.ensure_fast_client(supervisor)?;
                let result = self
                    .fast_client
                    .as_mut()
                    .expect("fast client was initialized")
                    .send_midi_with_offsets(events, patch);
                if result.is_err() {
                    // 次回の送信で張り直す。
                    self.fast_client = None;
                }
                result
            }
        }
    }

    pub(super) fn stop(&mut self, supervisor: &RealtimePlayServerSupervisor) -> Result<()> {
        match self.transport {
            KeyboardTransport::Http => supervisor.stop(),
            KeyboardTransport::SharedMemory => match self.fast_client.as_mut() {
                Some(client) => client.stop(),
                None => Ok(()),
            },
        }
    }

    pub(super) fn disconnect(&mut self) {
        self.fast_client = None;
    }

    fn ensure_fast_client(&mut self, supervisor: &RealtimePlayServerSupervisor) -> Result<()> {
        if self.fast_client.is_none() {
            supervisor.ensure_started_for_fast_midi()?;
            let mut client = connect_fast_client(supervisor.port())?;
            client.set_buffer_multiplier(BUFFER_MULTIPLIER)?;
            self.fast_client = Some(client);
        }
        Ok(())
    }
}

/// サーバー起動直後は共有メモリがまだ用意されていないことがあるため、少しリトライする。
#[cfg(windows)]
fn connect_fast_client(port: u16) -> Result<FastMidiClient> {
    use std::time::{Duration, Instant};

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
fn connect_fast_client(port: u16) -> Result<FastMidiClient> {
    FastMidiClient::connect(port)
}
