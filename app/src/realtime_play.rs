use std::{
    io::Read as _,
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _, Result};

use crate::config::Config;

mod live_buffer;
mod live_patch;
mod process;

use process::{
    default_realtime_play_server_executable_name, shell_command, sibling_realtime_play_server_path,
    stop_child,
};

const PLAY_SERVER_PLAY_PATH: &str = "/play";
const PLAY_SERVER_PLAY_MML_PATH: &str = "/play-mml";
const PLAY_SERVER_MIDI_PATH: &str = "/midi";
const PLAY_SERVER_STOP_PATH: &str = "/stop";
const PLAY_CONTENT_TYPE_MIDI: &str = "audio/midi";
const PLAY_CONTENT_TYPE_MML: &str = "text/plain; charset=utf-8";
const PLAY_CONTENT_TYPE_JSON: &str = "application/json";
const PLAY_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const PLAY_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const PLAY_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) struct RealtimePlayServerSupervisor {
    port: u16,
    command: String,
    agent: ureq::Agent,
    state: Mutex<PlayServerState>,
    live_buffer: Mutex<live_buffer::LiveBufferState>,
    next_request_id: AtomicU64,
    #[cfg(test)]
    spawn_count: AtomicU64,
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
    pub(crate) fn new(cfg: &Config) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(30))
            .timeout_write(Duration::from_secs(30))
            .build();
        Self {
            port: cfg.realtime_play_server_port,
            command: cfg.realtime_play_server_command.clone(),
            agent,
            state: Mutex::new(PlayServerState::default()),
            live_buffer: Mutex::new(live_buffer::LiveBufferState::default()),
            next_request_id: AtomicU64::new(1),
            #[cfg(test)]
            spawn_count: AtomicU64::new(0),
        }
    }

    pub(crate) fn play_smf(&self, smf_bytes: Vec<u8>) -> Result<()> {
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

    pub(crate) fn send_midi(&self, messages: &[[u8; 3]], patch: Option<&str>) -> Result<()> {
        let mut body = serde_json::json!({ "messages": messages });
        if let Some(patch) = patch {
            body["patch"] = serde_json::Value::String(patch.to_string());
        }
        let body = serde_json::to_vec(&body)?;
        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            let result = self
                .apply_live_buffer_once(server_generation)
                .and_then(|()| {
                    self.send_post_bytes(
                        PLAY_SERVER_MIDI_PATH,
                        Some((PLAY_CONTENT_TYPE_JSON, &body)),
                    )
                });
            match result {
                Ok(()) => return Ok(()),
                Err(PlayRequestError::Server { message, .. }) => return Err(anyhow!(message)),
                Err(PlayRequestError::Transport(message)) => {
                    retry += 1;
                    log_realtime_play_event(format!(
                        "action=midi retry={retry} transport_error=\"{}\"",
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
    pub(crate) fn play_mml(&self, mml: &str, fallback_smf: Vec<u8>) -> Result<()> {
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

    pub(crate) fn stop(&self) -> Result<()> {
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

    pub(crate) fn ensure_started_for_fast_midi(&self) -> Result<()> {
        self.ensure_started().map(|_| ())
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
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
        #[cfg(test)]
        self.spawn_count.fetch_add(1, Ordering::Relaxed);

        let mut command = self.build_command();
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn().map_err(|error| {
            anyhow!(
                "realtime play server の起動に失敗しました (command: {}): {}",
                self.command_description(),
                error
            )
        })
    }

    fn build_command(&self) -> Command {
        let trimmed = self.command.trim();
        if !trimmed.is_empty() {
            return shell_command(trimmed);
        }

        if let Some(path) = sibling_realtime_play_server_path() {
            return Command::new(path);
        }
        Command::new(default_realtime_play_server_executable_name())
    }

    fn command_description(&self) -> String {
        let trimmed = self.command.trim();
        if trimmed.is_empty() {
            default_realtime_play_server_executable_name().to_string()
        } else {
            trimmed.to_string()
        }
    }

    #[cfg(test)]
    fn spawn_count_for_test(&self) -> u64 {
        self.spawn_count.load(Ordering::Relaxed)
    }
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

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index == max_chars {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

fn log_realtime_play_event(message: impl Into<String>) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("realtime-play: {}", message.into()));
    #[cfg(test)]
    let _ = message.into();
}

#[cfg(test)]
#[path = "realtime_play/tests.rs"]
mod tests;
