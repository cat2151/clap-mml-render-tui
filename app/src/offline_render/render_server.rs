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
}

#[derive(Default)]
struct RenderServerState {
    child: Option<Child>,
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
        }
    }

    pub(super) fn render_mml(&self, mml: &str) -> Result<OfflineRenderOutput> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_offline_render_event(format!(
            "backend=render_server request_id={request_id} retry=0 mml_hash={}",
            crate::history::daw_cache_mml_hash(mml)
        ));

        self.ensure_started()?;
        match self.send_once(mml) {
            Ok(samples) => Ok(OfflineRenderOutput {
                samples,
                patch_name: RENDER_SERVER_PATCH_NAME.to_string(),
            }),
            Err(RenderRequestError::Server(message)) => Err(anyhow!(message)),
            Err(RenderRequestError::Transport(message)) => {
                log_offline_render_event(format!(
                    "backend=render_server request_id={request_id} retry=1 restart_reason=\"{}\"",
                    truncate_for_log(&message, 160)
                ));
                self.restart()?;
                match self.send_once(mml) {
                    Ok(samples) => Ok(OfflineRenderOutput {
                        samples,
                        patch_name: RENDER_SERVER_PATCH_NAME.to_string(),
                    }),
                    Err(RenderRequestError::Server(message)) => Err(anyhow!(message)),
                    Err(RenderRequestError::Transport(message)) => Err(anyhow!(
                        "render-server request failed after retry: {}",
                        message
                    )),
                }
            }
        }
    }

    fn ensure_started(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        self.drop_exited_child_locked(&mut state)?;
        if self.port_accepts_connections() {
            return Ok(());
        }
        if state.child.is_none() {
            state.child = Some(self.spawn_child()?);
        }
        self.wait_for_port_locked(&mut state)
    }

    fn restart(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        stop_child(state.child.take());
        state.child = Some(self.spawn_child()?);
        self.wait_for_port_locked(&mut state)
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

    fn wait_for_port_locked(&self, state: &mut RenderServerState) -> Result<()> {
        let deadline = Instant::now() + RENDER_SERVER_START_TIMEOUT;
        loop {
            self.drop_exited_child_locked(state)?;
            if self.port_accepts_connections() {
                return Ok(());
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
        }
        Ok(())
    }

    fn port_accepts_connections(&self) -> bool {
        TcpStream::connect_timeout(&self.socket_addr(), RENDER_SERVER_CONNECT_TIMEOUT).is_ok()
    }

    fn socket_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }

    fn spawn_child(&self) -> Result<Child> {
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
