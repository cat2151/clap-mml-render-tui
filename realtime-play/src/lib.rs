//! realtime play server（外部プロセス）の監督と、そこへの MIDI / MML 送信。
//!
//! notepad 再生・DAW・keyboard 画面が共有する再生インフラ。`RealtimePlayServerSupervisor`
//! がサーバープロセスの起動・再起動・HTTP リクエストを担い、`fast_midi_ipc` が
//! Windows の共有メモリ経由の低レイテンシ MIDI 送信を担う。

use std::{
    io::Read as _,
    net::{SocketAddr, TcpStream},
    process::{Child, Command},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _, Result};

use cmrt_runtime::Config;

pub mod fast_midi_ipc;
mod live_ipc;
mod logging;
mod process;

pub use logging::set_log_sink;

use logging::{log_realtime_play_event, truncate_for_log};

pub use fast_midi_ipc::{FastMidiEvent, InstanceId, LimiterMeter, INSTANCE_COUNT};
pub use live_ipc::{PatchVoicing, VoicingReport};

use process::{build_realtime_play_server_command, spawn_realtime_play_server, stop_child};

/// UI が見せるトラック数の既定値。サーバーが生成する instance 数はこの 2 倍になる。
pub const DEFAULT_LIVE_INSTANCE_COUNT: usize = INSTANCE_COUNT / BANK_COUNT;
/// UI が見せるトラック数（grid sequencer の `t` キーが循環する値）。
pub const SUPPORTED_LIVE_INSTANCE_COUNTS: [usize; 5] = [1, 2, 4, 8, 16];
/// サーバーへ要求できる instance 数。トラック数それぞれの bank 2 本ぶん。
pub const SUPPORTED_SERVER_INSTANCE_COUNTS: [usize; 6] = [1, 2, 4, 8, 16, 32];
pub const LIVE_INSTANCE_COUNT_ENV: &str = "CMRT_LIVE_INSTANCE_COUNT";

/// bank の数。grid sequencer の chord mode は、鳴っている bank の裏でもう一方へ
/// 次の patch を先読みし、小節境界で入れ替えることで差し替えの無音をなくす。
pub const BANK_COUNT: usize = 2;

/// トラック数に対して、サーバーが生成すべき instance 数（= bank 2 本ぶん）。
pub fn server_instance_count(track_count: usize) -> usize {
    normalize_live_instance_count(track_count) * BANK_COUNT
}

const PLAY_SERVER_PLAY_PATH: &str = "/play";
const PLAY_SERVER_PLAY_MML_PATH: &str = "/play-mml";
const PLAY_SERVER_STOP_PATH: &str = "/stop";
const PLAY_CONTENT_TYPE_MIDI: &str = "audio/midi";
const PLAY_CONTENT_TYPE_MML: &str = "text/plain; charset=utf-8";
const PLAY_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const PLAY_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const PLAY_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct RealtimePlayServerSupervisor {
    port: u16,
    command: String,
    live_instance_count: usize,
    agent: ureq::Agent,
    state: Mutex<PlayServerState>,
    fast_client: Mutex<Option<fast_midi_ipc::FastMidiClient>>,
    live_buffer_multiplier: Mutex<u8>,
    startup_progress: Arc<Mutex<Option<RealtimePlayServerStartupProgress>>>,
    next_request_id: AtomicU64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RealtimePlayServerStartupProgress {
    pub initialized_instances: usize,
    pub total_instances: usize,
}

#[derive(Default)]
struct PlayServerState {
    child: Option<Child>,
    generation: u64,
}

enum PlayRequestError {
    Server { status: u16, message: String },
    Transport(String),
}

impl RealtimePlayServerSupervisor {
    pub fn new(cfg: &Config) -> Self {
        Self::with_live_instance_count(cfg, server_instance_count(DEFAULT_LIVE_INSTANCE_COUNT))
    }

    /// `live_instance_count` は**サーバーが生成する instance 数**であって、UI が見せる
    /// トラック数ではない。トラック数から求めるには [`server_instance_count`] を使う。
    pub fn with_live_instance_count(cfg: &Config, live_instance_count: usize) -> Self {
        assert!(
            SUPPORTED_SERVER_INSTANCE_COUNTS.contains(&live_instance_count),
            "server instance count must be one of {SUPPORTED_SERVER_INSTANCE_COUNTS:?}"
        );
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(30))
            .timeout_write(Duration::from_secs(30))
            .build();
        Self {
            port: cfg.realtime_play_server_port,
            command: cfg.realtime_play_server_command.clone(),
            live_instance_count,
            agent,
            state: Mutex::new(PlayServerState::default()),
            fast_client: Mutex::new(None),
            live_buffer_multiplier: Mutex::new(4),
            startup_progress: Arc::new(Mutex::new(None)),
            next_request_id: AtomicU64::new(1),
        }
    }

    pub fn play_smf(&self, smf_bytes: Vec<u8>) -> Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_realtime_play_event(format!(
            "request_id={request_id} action=play retry=0 bytes={}",
            smf_bytes.len()
        ));

        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_play_once(&smf_bytes) {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(message)) => {
                    retry += 1;
                    log_realtime_play_event(format!(
                        "request_id={request_id} action=play retry={retry} transport_error=\"{}\"",
                        truncate_for_log(&message, 160)
                    ));
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    /// MML（行頭の音色 JSON 込み）を /play-mml へ送る。
    /// 旧サーバー（/play-mml 未対応）の場合は /play へ SMF でフォールバックする
    /// （音色は反映されないが従来どおり再生される）。
    pub fn play_mml(&self, mml: &str, fallback_smf: Vec<u8>) -> Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_realtime_play_event(format!(
            "request_id={request_id} action=play-mml retry=0 chars={}",
            mml.chars().count()
        ));

        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_play_mml_once(mml) {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server {
                    status: 404 | 405, ..
                }) => {
                    log_realtime_play_event(format!(
                        "request_id={request_id} action=play-mml fallback=play \
                         reason=\"play server does not support /play-mml (update the server)\""
                    ));
                    return self.play_smf(fallback_smf);
                }
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(message)) => {
                    retry += 1;
                    log_realtime_play_event(format!(
                        "request_id={request_id} action=play-mml retry={retry} transport_error=\"{}\"",
                        truncate_for_log(&message, 160)
                    ));
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
    }

    pub fn stop(&self) -> Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let Some(_server_generation) = self.running_server_generation()? else {
            log_realtime_play_event(format!(
                "request_id={request_id} action=stop skipped=no-server"
            ));
            return Ok(());
        };

        match self.send_stop_once() {
            Ok(()) => Ok(()),
            Err(PlayRequestError::Server { message, .. }) => Err(anyhow!(message)),
            Err(PlayRequestError::Transport(message)) => {
                log_realtime_play_event(format!(
                    "request_id={request_id} action=stop transport_error=\"{}\"",
                    truncate_for_log(&message, 160)
                ));
                Ok(())
            }
        }
    }

    fn running_server_generation(&self) -> Result<Option<u64>> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;
        if self.port_accepts_connections() {
            return Ok(Some(state.generation));
        }
        if state.child.is_none() {
            return Ok(None);
        }
        self.wait_for_port_locked(&mut state).map(Some)
    }

    fn ensure_started(&self) -> Result<u64> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;
        if self.port_accepts_connections() {
            return Ok(state.generation);
        }
        if state.child.is_none() {
            self.spawn_child_locked(&mut state)?;
        }
        self.wait_for_port_locked(&mut state)
    }

    pub fn ensure_started_for_fast_midi(&self) -> Result<()> {
        self.ensure_started().map(|_| ())
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn live_instance_count(&self) -> usize {
        self.live_instance_count
    }

    pub fn startup_progress(&self) -> Option<RealtimePlayServerStartupProgress> {
        *self.startup_progress.lock().unwrap()
    }

    fn recover_after_transport_failure(&self, failed_generation: u64) -> Result<u64> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;

        if state.generation == failed_generation {
            self.restart_locked(&mut state)?;
        } else if state.child.is_none() && !self.port_accepts_connections() {
            self.spawn_child_locked(&mut state)?;
        }

        self.wait_for_port_locked(&mut state)
    }

    fn restart_locked(&self, state: &mut PlayServerState) -> Result<()> {
        stop_child(state.child.take());
        self.bump_generation_locked(state);
        self.spawn_child_locked(state)
    }

    fn spawn_child_locked(&self, state: &mut PlayServerState) -> Result<()> {
        state.child = Some(self.spawn_child()?);
        self.bump_generation_locked(state);
        Ok(())
    }

    fn bump_generation_locked(&self, state: &mut PlayServerState) {
        state.generation = state.generation.wrapping_add(1);
        if state.generation == 0 {
            state.generation = 1;
        }
    }

    fn wait_for_port_locked(&self, state: &mut PlayServerState) -> Result<u64> {
        let deadline = Instant::now() + PLAY_SERVER_START_TIMEOUT;
        loop {
            self.drop_exited_child_locked(state)?;
            if self.port_accepts_connections() {
                return Ok(state.generation);
            }
            if state.child.is_none() {
                self.spawn_child_locked(state)?;
            }
            if Instant::now() >= deadline {
                anyhow::bail!(
                    "realtime play server did not start listening on 127.0.0.1:{} within {:?}",
                    self.port,
                    PLAY_SERVER_START_TIMEOUT
                );
            }
            std::thread::sleep(PLAY_SERVER_START_POLL_INTERVAL);
        }
    }

    fn drop_exited_child_locked(&self, state: &mut PlayServerState) -> Result<()> {
        let Some(child) = state.child.as_mut() else {
            return Ok(());
        };
        if child
            .try_wait()
            .with_context(|| "realtime play server child status check failed")?
            .is_some()
        {
            state.child = None;
            self.bump_generation_locked(state);
        }
        Ok(())
    }

    fn send_play_once(&self, smf_bytes: &[u8]) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(
            PLAY_SERVER_PLAY_PATH,
            Some((PLAY_CONTENT_TYPE_MIDI, smf_bytes)),
        )
    }

    fn send_play_mml_once(&self, mml: &str) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(
            PLAY_SERVER_PLAY_MML_PATH,
            Some((PLAY_CONTENT_TYPE_MML, mml.as_bytes())),
        )
    }

    fn send_stop_once(&self) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(PLAY_SERVER_STOP_PATH, None)
    }

    fn send_post_bytes(
        &self,
        path: &str,
        body: Option<(&str, &[u8])>,
    ) -> std::result::Result<(), PlayRequestError> {
        let url = format!("http://127.0.0.1:{}{}", self.port, path);
        let request = self.agent.post(&url);
        let response = match body {
            Some((content_type, body)) => {
                request.set("Content-Type", content_type).send_bytes(body)
            }
            None => request.send_bytes(&[]),
        };
        match response {
            Ok(response) if (200..300).contains(&response.status()) => Ok(()),
            Ok(response) => Err(PlayRequestError::Server {
                status: response.status(),
                message: format!("realtime play server returned HTTP {}", response.status()),
            }),
            Err(ureq::Error::Status(status, response)) => {
                let body = response_body(response);
                let body = body.trim();
                let message = if body.is_empty() {
                    format!("realtime play server returned HTTP {status}")
                } else {
                    format!("realtime play server returned HTTP {status}: {body}")
                };
                Err(PlayRequestError::Server { status, message })
            }
            Err(ureq::Error::Transport(error)) => {
                Err(PlayRequestError::Transport(error.to_string()))
            }
        }
    }

    fn port_accepts_connections(&self) -> bool {
        TcpStream::connect_timeout(&self.socket_addr(), PLAY_SERVER_CONNECT_TIMEOUT).is_ok()
    }

    fn socket_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }

    fn spawn_child(&self) -> Result<Child> {
        let (command, launch_description) = self.build_command();
        spawn_realtime_play_server(
            command,
            &launch_description,
            self.port,
            self.live_instance_count,
            Arc::clone(&self.startup_progress),
        )
    }

    fn build_command(&self) -> (Command, String) {
        let (mut command, description) = build_realtime_play_server_command(&self.command);
        command.env(
            LIVE_INSTANCE_COUNT_ENV,
            self.live_instance_count.to_string(),
        );
        (command, description)
    }
}

pub fn normalize_live_instance_count(count: usize) -> usize {
    if SUPPORTED_LIVE_INSTANCE_COUNTS.contains(&count) {
        count
    } else {
        DEFAULT_LIVE_INSTANCE_COUNT
    }
}

pub fn next_live_instance_count(current: usize) -> usize {
    let current = normalize_live_instance_count(current);
    let index = SUPPORTED_LIVE_INSTANCE_COUNTS
        .iter()
        .position(|count| *count == current)
        .expect("normalized live instance count is supported");
    SUPPORTED_LIVE_INSTANCE_COUNTS[(index + 1) % SUPPORTED_LIVE_INSTANCE_COUNTS.len()]
}

impl Drop for RealtimePlayServerSupervisor {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            stop_child(state.child.take());
        }
    }
}

fn response_body(response: ureq::Response) -> String {
    let mut body = String::new();
    let _ = response.into_reader().read_to_string(&mut body);
    body
}

#[cfg(test)]
mod tests;
