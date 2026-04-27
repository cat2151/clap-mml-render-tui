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

const PLAY_SERVER_PLAY_PATH: &str = "/play";
const PLAY_SERVER_STOP_PATH: &str = "/stop";
const PLAY_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const PLAY_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const PLAY_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) struct RealtimePlayServerSupervisor {
    port: u16,
    command: String,
    agent: ureq::Agent,
    state: Mutex<PlayServerState>,
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
    Server(String),
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
                Err(PlayRequestError::Server(message)) => return Err(anyhow!(message)),
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
            Err(PlayRequestError::Server(message)) => Err(anyhow!(message)),
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
        self.send_post_bytes(PLAY_SERVER_PLAY_PATH, Some(smf_bytes))
    }

    fn send_stop_once(&self) -> std::result::Result<(), PlayRequestError> {
        self.send_post_bytes(PLAY_SERVER_STOP_PATH, None)
    }

    fn send_post_bytes(
        &self,
        path: &str,
        body: Option<&[u8]>,
    ) -> std::result::Result<(), PlayRequestError> {
        let url = format!("http://127.0.0.1:{}{}", self.port, path);
        let request = self.agent.post(&url);
        let response = match body {
            Some(body) => request.set("Content-Type", "audio/midi").send_bytes(body),
            None => request.send_bytes(&[]),
        };
        match response {
            Ok(response) if (200..300).contains(&response.status()) => Ok(()),
            Ok(response) => Err(PlayRequestError::Server(format!(
                "realtime play server returned HTTP {}",
                response.status()
            ))),
            Err(ureq::Error::Status(status, response)) => {
                let body = response_body(response);
                let body = body.trim();
                let message = if body.is_empty() {
                    format!("realtime play server returned HTTP {status}")
                } else {
                    format!("realtime play server returned HTTP {status}: {body}")
                };
                Err(PlayRequestError::Server(message))
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

fn stop_child(child: Option<Child>) {
    let Some(mut child) = child else {
        return;
    };
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn sibling_realtime_play_server_path() -> Option<std::path::PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let sibling = current_exe
        .parent()?
        .join(default_realtime_play_server_executable_name());
    sibling.is_file().then_some(sibling)
}

fn default_realtime_play_server_executable_name() -> &'static str {
    if cfg!(windows) {
        "clap-mml-realtime-play-server.exe"
    } else {
        "clap-mml-realtime-play-server"
    }
}

#[cfg(target_os = "windows")]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
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
mod tests {
    use std::{
        io::{BufRead as _, BufReader, Read as _, Write as _},
        net::{TcpListener, TcpStream},
        sync::mpsc,
    };

    use super::*;

    #[derive(Debug)]
    struct CapturedRequest {
        method: String,
        path: String,
        content_type: Option<String>,
        body: Vec<u8>,
    }

    fn cfg_for_port(port: u16) -> Config {
        Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 48_000.0,
            buffer_size: 512,
            patches_dirs: None,
            offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
            offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
            offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
            offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
            offline_render_server_command: String::new(),
            realtime_audio_backend: crate::config::RealtimeAudioBackend::PlayServer,
            realtime_play_server_port: port,
            realtime_play_server_command: "exit 0".to_string(),
        }
    }

    fn spawn_one_request_server(
        status_line: &'static str,
        body: &'static str,
    ) -> (u16, mpsc::Receiver<CapturedRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                write!(
                    stream,
                    "{status_line}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                tx.send(request).unwrap();
                break;
            }
        });
        (port, rx)
    }

    fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
        let mut reader = BufReader::new(stream);
        let mut first_line = String::new();
        if reader.read_line(&mut first_line).ok()? == 0 {
            return None;
        }
        if first_line.trim().is_empty() {
            return None;
        }
        let mut parts = first_line.split_whitespace();
        let method = parts.next()?.to_string();
        let path = parts.next()?.to_string();
        let mut content_length = 0usize;
        let mut content_type = None;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).ok()?;
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = value.trim().parse().unwrap();
            } else if name.eq_ignore_ascii_case("Content-Type") {
                content_type = Some(value.trim().to_string());
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        Some(CapturedRequest {
            method,
            path,
            content_type,
            body,
        })
    }

    #[test]
    fn play_smf_posts_binary_body_to_play_endpoint() {
        let (port, rx) = spawn_one_request_server("HTTP/1.1 202 Accepted", "accepted");
        let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

        supervisor.play_smf(vec![0, 1, 2, 255]).unwrap();

        let request = rx.recv().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, PLAY_SERVER_PLAY_PATH);
        assert_eq!(request.content_type.as_deref(), Some("audio/midi"));
        assert_eq!(request.body, vec![0, 1, 2, 255]);
        assert_eq!(supervisor.spawn_count_for_test(), 0);
    }

    #[test]
    fn stop_posts_to_stop_endpoint_without_spawning_when_server_is_listening() {
        let (port, rx) = spawn_one_request_server("HTTP/1.1 204 No Content", "");
        let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

        supervisor.stop().unwrap();

        let request = rx.recv().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, PLAY_SERVER_STOP_PATH);
        assert!(request.body.is_empty());
        assert_eq!(supervisor.spawn_count_for_test(), 0);
    }

    #[test]
    fn stop_without_running_server_does_not_spawn_child() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

        supervisor.stop().unwrap();

        assert_eq!(supervisor.spawn_count_for_test(), 0);
    }

    #[test]
    fn server_error_body_is_returned() {
        let (port, _rx) =
            spawn_one_request_server("HTTP/1.1 415 Unsupported Media Type", "bad type");
        let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

        let error = supervisor.play_smf(vec![0]).unwrap_err();

        assert!(error
            .to_string()
            .contains("realtime play server returned HTTP 415: bad type"));
    }
}
