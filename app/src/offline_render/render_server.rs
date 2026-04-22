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

use super::{
    decode_wav_bytes, log_offline_render_event, truncate_for_log, Config, OfflineRenderOutput,
    RENDER_SERVER_CONNECT_TIMEOUT, RENDER_SERVER_PATCH_NAME, RENDER_SERVER_PATH,
    RENDER_SERVER_START_POLL_INTERVAL, RENDER_SERVER_START_TIMEOUT,
};

pub(super) struct RenderServerSupervisor {
    port: u16,
    command: String,
    expected_sample_rate: u32,
    agent: ureq::Agent,
    state: Mutex<RenderServerState>,
    next_request_id: AtomicU64,
    #[cfg(test)]
    spawn_count: AtomicU64,
}

#[derive(Default)]
struct RenderServerState {
    child: Option<Child>,
    generation: u64,
}

enum RenderRequestError {
    Server(String),
    Transport(String),
}

impl RenderServerSupervisor {
    pub(super) fn new(cfg: &Config) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(120))
            .timeout_write(Duration::from_secs(120))
            .build();
        Self {
            port: cfg.offline_render_server_port,
            command: cfg.offline_render_server_command.clone(),
            expected_sample_rate: cfg.sample_rate as u32,
            agent,
            state: Mutex::new(RenderServerState::default()),
            next_request_id: AtomicU64::new(1),
            #[cfg(test)]
            spawn_count: AtomicU64::new(0),
        }
    }

    pub(super) fn render_mml(&self, mml: &str) -> Result<OfflineRenderOutput> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_offline_render_event(format!(
            "backend=render_server request_id={request_id} retry=0 mml_hash={}",
            crate::history::daw_cache_mml_hash(mml)
        ));

        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            match self.send_once(mml) {
                Ok(samples) => {
                    return Ok(OfflineRenderOutput {
                        samples,
                        patch_name: RENDER_SERVER_PATCH_NAME.to_string(),
                    });
                }
                Err(RenderRequestError::Server(message)) => return Err(anyhow!(message)),
                Err(RenderRequestError::Transport(message)) => {
                    retry += 1;
                    log_offline_render_event(format!(
                        "backend=render_server request_id={request_id} retry={retry} transport_error=\"{}\"",
                        truncate_for_log(&message, 160)
                    ));
                    self.recover_after_transport_failure(server_generation)?;
                }
            }
        }
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

    fn restart_locked(&self, state: &mut RenderServerState) -> Result<()> {
        stop_child(state.child.take());
        self.bump_generation_locked(state);
        self.spawn_child_locked(state)
    }

    fn spawn_child_locked(&self, state: &mut RenderServerState) -> Result<()> {
        state.child = Some(self.spawn_child()?);
        self.bump_generation_locked(state);
        Ok(())
    }

    fn bump_generation_locked(&self, state: &mut RenderServerState) {
        state.generation = state.generation.wrapping_add(1);
        if state.generation == 0 {
            state.generation = 1;
        }
    }

    fn wait_for_port_locked(&self, state: &mut RenderServerState) -> Result<u64> {
        let deadline = Instant::now() + RENDER_SERVER_START_TIMEOUT;
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
                    "render-server did not start listening on 127.0.0.1:{} within {:?}",
                    self.port,
                    RENDER_SERVER_START_TIMEOUT
                );
            }
            std::thread::sleep(RENDER_SERVER_START_POLL_INTERVAL);
        }
    }

    fn drop_exited_child_locked(&self, state: &mut RenderServerState) -> Result<()> {
        let Some(child) = state.child.as_mut() else {
            return Ok(());
        };
        if child
            .try_wait()
            .with_context(|| "render-server child status check failed")?
            .is_some()
        {
            state.child = None;
            self.bump_generation_locked(state);
        }
        Ok(())
    }

    fn send_once(&self, mml: &str) -> std::result::Result<Vec<f32>, RenderRequestError> {
        let url = format!("http://127.0.0.1:{}{}", self.port, RENDER_SERVER_PATH);
        let response = self
            .agent
            .post(&url)
            .set("Content-Type", "text/plain; charset=utf-8")
            .send_string(mml);
        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                let body = response.into_string().unwrap_or_default();
                let body = body.trim();
                let message = if body.is_empty() {
                    format!("render-server returned HTTP {status}")
                } else {
                    format!("render-server returned HTTP {status}: {body}")
                };
                return Err(RenderRequestError::Server(message));
            }
            Err(ureq::Error::Transport(error)) => {
                return Err(RenderRequestError::Transport(error.to_string()));
            }
        };

        let content_type = response
            .header("Content-Type")
            .unwrap_or_default()
            .to_string();
        if !content_type
            .split(';')
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("audio/wav"))
        {
            let body = response.into_string().unwrap_or_default();
            return Err(RenderRequestError::Server(format!(
                "render-server returned unexpected Content-Type '{content_type}': {}",
                body.trim()
            )));
        }

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|error| RenderRequestError::Transport(error.to_string()))?;
        decode_wav_bytes(&bytes, self.expected_sample_rate)
            .map_err(|error| RenderRequestError::Server(error.to_string()))
    }

    fn port_accepts_connections(&self) -> bool {
        TcpStream::connect_timeout(&self.socket_addr(), RENDER_SERVER_CONNECT_TIMEOUT).is_ok()
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
                "render-server の起動に失敗しました (command: {}): {}",
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

        if let Some(path) = sibling_render_server_path() {
            return Command::new(path);
        }
        Command::new(default_render_server_executable_name())
    }

    fn command_description(&self) -> String {
        let trimmed = self.command.trim();
        if trimmed.is_empty() {
            default_render_server_executable_name().to_string()
        } else {
            trimmed.to_string()
        }
    }

    #[cfg(test)]
    fn set_generation_for_test(&self, generation: u64) {
        self.state.lock().unwrap().generation = generation;
    }

    #[cfg(test)]
    fn spawn_count_for_test(&self) -> u64 {
        self.spawn_count.load(Ordering::Relaxed)
    }
}

impl Drop for RenderServerSupervisor {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            stop_child(state.child.take());
        }
    }
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

fn sibling_render_server_path() -> Option<std::path::PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let sibling = current_exe
        .parent()?
        .join(default_render_server_executable_name());
    sibling.is_file().then_some(sibling)
}

fn default_render_server_executable_name() -> &'static str {
    if cfg!(windows) {
        "clap-mml-render-server.exe"
    } else {
        "clap-mml-render-server"
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

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    fn supervisor_for_listening_port(listener: &TcpListener) -> RenderServerSupervisor {
        let port = listener.local_addr().unwrap().port();
        let cfg: Config = toml::from_str(&format!(
            r#"
plugin_path = "dummy.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 48000
buffer_size = 512
offline_render_backend = "render_server"
offline_render_server_port = {port}
offline_render_server_command = "exit 0"
"#
        ))
        .unwrap();
        RenderServerSupervisor::new(&cfg)
    }

    #[test]
    fn stale_transport_failure_reuses_newer_generation_without_restart() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let supervisor = supervisor_for_listening_port(&listener);
        supervisor.set_generation_for_test(2);

        let generation = supervisor.recover_after_transport_failure(1).unwrap();

        assert_eq!(generation, 2);
        assert_eq!(supervisor.spawn_count_for_test(), 0);
    }

    #[test]
    fn current_generation_transport_failure_restarts_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let supervisor = supervisor_for_listening_port(&listener);
        supervisor.set_generation_for_test(7);

        let generation = supervisor.recover_after_transport_failure(7).unwrap();

        assert!(generation > 7);
        assert_eq!(supervisor.spawn_count_for_test(), 1);
    }
}
