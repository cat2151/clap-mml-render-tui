use std::{
    io::{BufRead as _, BufReader, Read},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, ExitStatus, Stdio},
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
use command_resolution::{resolve_render_server_command, ResolvedRenderServerCommand};

mod command_resolution;

const EXIT_ON_STDIN_CLOSE_ENV: &str = "CMRT_RENDER_SERVER_EXIT_ON_STDIN_CLOSE";
/// `1` なら transport error でも render-server を kill せず、その render を失敗させて
/// プロセスを生かしたままにする（診断用。生かした pid にデバッガやダンプツールを当てる）。
const NEVER_KILL_ENV: &str = "CMRT_RENDER_SERVER_NEVER_KILL";

pub(super) struct RenderServerSupervisor {
    port: u16,
    /// 起動コマンドの解決結果。**supervisor 生成時の 1 度だけ**決める（ADR 0017 と同じ。
    /// 起こし直しで実体が変わらない）。見つからなければ `Err`（探した場所の説明）を持ち、
    /// spawn のたびにそれを返す。PATH へは絶対に落ちない。
    resolved_command: Result<ResolvedRenderServerCommand, String>,
    /// 子へ `--config` として渡す path（`Config::source_path`）。
    config_path: Option<PathBuf>,
    expected_sample_rate: u32,
    agent: ureq::Agent,
    state: Mutex<RenderServerState>,
    next_request_id: AtomicU64,
    never_kill: bool,
    #[cfg(test)]
    spawn_count: AtomicU64,
    #[cfg(test)]
    restart_count: AtomicU64,
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
        // http_status_as_error(false): 4xx/5xx も Ok として受け取り、
        // 本文をエラーメッセージへ載せられるようにする（ureq 3 の StatusCode error は本文を持たない）。
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_send_body(Some(Duration::from_secs(120)))
                .timeout_recv_response(Some(Duration::from_secs(120)))
                .timeout_recv_body(Some(Duration::from_secs(120)))
                .http_status_as_error(false)
                .build(),
        );
        let resolved_command = resolve_render_server_command(&cfg.offline_render_server_command);
        if let Ok(resolved) = &resolved_command {
            log_offline_render_event(render_server_resolved_log_message(
                resolved,
                cfg.source_path.as_deref(),
            ));
        }
        Self {
            port: cfg.offline_render_server_port,
            resolved_command,
            config_path: cfg.source_path.clone(),
            expected_sample_rate: cfg.sample_rate as u32,
            agent,
            state: Mutex::new(RenderServerState::default()),
            next_request_id: AtomicU64::new(1),
            never_kill: never_kill_requested(),
            #[cfg(test)]
            spawn_count: AtomicU64::new(0),
            #[cfg(test)]
            restart_count: AtomicU64::new(0),
        }
    }

    pub(super) fn render_mml(&self, mml: &str) -> Result<OfflineRenderOutput> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        log_offline_render_event(format!(
            "backend=render_server request_id={request_id} retry=0 mml_hash={}",
            cmrt_history::daw_cache_mml_hash(mml)
        ));

        let mut retry = 0;
        loop {
            let server_generation = self.ensure_started()?;
            let sent_at = Instant::now();
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
                        "backend=render_server request_id={request_id} retry={retry} generation={server_generation} elapsed_ms={} transport_error=\"{}\"",
                        sent_at.elapsed().as_millis(),
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
            if self.never_kill {
                let pid = state.child.as_ref().map(Child::id);
                log_offline_render_event(format!(
                    "backend=render_server event=never-kill-hold pid={} generation={}",
                    pid.map_or_else(|| "none".to_string(), |pid| pid.to_string()),
                    state.generation
                ));
                anyhow::bail!(
                    "render-server との通信に失敗しましたが {NEVER_KILL_ENV}=1 のため kill せず生かしています (pid={})",
                    pid.map_or_else(|| "none".to_string(), |pid| pid.to_string())
                );
            }
            self.restart_locked(&mut state)?;
        } else if state.child.is_none() && !self.port_accepts_connections() {
            self.spawn_child_locked(&mut state)?;
        }

        self.wait_for_port_locked(&mut state)
    }

    fn restart_locked(&self, state: &mut RenderServerState) -> Result<()> {
        #[cfg(test)]
        self.restart_count.fetch_add(1, Ordering::Relaxed);
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
        let pid = child.id();
        if let Some(status) = child
            .try_wait()
            .with_context(|| "render-server child status check failed")?
        {
            // 通常終了（コード 0）以外は、クラッシュの直接証拠（exit code）として残す。
            // Windows では access violation 等の NTSTATUS がそのまま code に反映されることが多い
            // （例: STATUS_ACCESS_VIOLATION = 0xC0000005 → code=-1073741819）。
            log_offline_render_event(format!(
                "backend=render_server event=server-exited pid={pid} {}",
                exit_status_log_fields(&status)
            ));
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
            .header("Content-Type", "text/plain; charset=utf-8")
            .send(mml);
        let mut response = match response {
            Ok(response) => response,
            Err(error) => {
                return Err(RenderRequestError::Transport(error.to_string()));
            }
        };
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.body_mut().read_to_string().unwrap_or_default();
            let body = body.trim();
            let message = if body.is_empty() {
                format!("render-server returned HTTP {status}")
            } else {
                format!("render-server returned HTTP {status}: {body}")
            };
            return Err(RenderRequestError::Server(message));
        }

        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        if !content_type
            .split(';')
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("audio/wav"))
        {
            let body = response.body_mut().read_to_string().unwrap_or_default();
            return Err(RenderRequestError::Server(format!(
                "render-server returned unexpected Content-Type '{content_type}': {}",
                body.trim()
            )));
        }

        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
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

        let resolved = match &self.resolved_command {
            Ok(resolved) => resolved,
            Err(message) => return Err(anyhow!(message.clone())),
        };
        let mut command = resolved.build_command(self.config_path.as_deref());
        command
            .env(EXIT_ON_STDIN_CLOSE_ENV, "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            anyhow!(
                "render-server の起動に失敗しました (command: {}): {}",
                resolved.describe(),
                error
            )
        })?;
        let pid = child.id();
        if let Some(stderr) = child.stderr.take() {
            spawn_render_server_stderr_logger(stderr, pid);
        }
        Ok(child)
    }

    #[cfg(test)]
    fn set_generation_for_test(&self, generation: u64) {
        self.state.lock().unwrap().generation = generation;
    }

    #[cfg(test)]
    fn spawn_count_for_test(&self) -> u64 {
        self.spawn_count.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    fn restart_count_for_test(&self) -> u64 {
        self.restart_count.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    fn set_never_kill_for_test(&mut self, never_kill: bool) {
        self.never_kill = never_kill;
    }

    /// 子が終了していれば `server-exited` を exit code つきでログに出す。
    #[cfg(test)]
    fn log_child_status_for_test(&self) {
        let mut state = self.state.lock().unwrap();
        let _ = self.drop_exited_child_locked(&mut state);
    }
}

fn render_server_resolved_log_message(
    resolved: &ResolvedRenderServerCommand,
    config_path: Option<&Path>,
) -> String {
    let mut message = format!(
        "render-server: exe={} (source={})",
        resolved.describe(),
        resolved.source_label()
    );
    if let Some(config_path) = resolved.forwarded_config_path(config_path) {
        message.push_str(&format!(" config={}", config_path.display()));
    }
    message
}

fn never_kill_requested() -> bool {
    std::env::var_os(NEVER_KILL_ENV).as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn spawn_render_server_stderr_logger(stderr: impl Read + Send + 'static, pid: u32) {
    let result = std::thread::Builder::new()
        .name("offline-render-server-stderr".to_string())
        .spawn(move || {
            for line in BufReader::new(stderr).lines() {
                match line {
                    Ok(line) => log_offline_render_event(render_server_stderr_log_message(
                        pid, &line,
                    )),
                    Err(error) => {
                        log_offline_render_event(format!(
                            "backend=render_server event=server-stderr-read-error pid={pid} error={error:?}"
                        ));
                        break;
                    }
                }
            }
        });
    if let Err(error) = result {
        log_offline_render_event(format!(
            "backend=render_server event=server-stderr-reader-start-error pid={pid} error={error:?}"
        ));
    }
}

/// exit status を人が読める形（成功可否・code・16進）にする。
/// Windows の異常終了は code に NTSTATUS がそのまま入ることが多く、16進のほうが
/// STATUS_ACCESS_VIOLATION (0xC0000005) 等の既知の値と照合しやすい。
fn exit_status_log_fields(status: &ExitStatus) -> String {
    match status.code() {
        Some(code) => format!(
            "success={} code={code} code_hex=0x{:08X}",
            status.success(),
            code as u32
        ),
        None => format!("success={} code=none", status.success()),
    }
}

fn render_server_stderr_log_message(pid: u32, line: &str) -> String {
    format!(
        "backend=render_server event=server-stderr pid={pid} line=\"{}\"",
        truncate_for_log(line, 1_000)
    )
}

impl Drop for RenderServerSupervisor {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            stop_child(state.child.take());
        }
    }
}

/// 子を止め、回収した exit status を残す。
///
/// `exited_before_kill=true` なら、こちらが kill する前にプロセスは既に終わっていた
/// （`server-exited` のログを出す前に transport error 側が先に走った）ということで、
/// その `code` はクラッシュの exit code そのもの。`false` なら kill によるもの。
fn stop_child(child: Option<Child>) {
    let Some(mut child) = child else {
        return;
    };
    let pid = child.id();
    let exited_before_kill = child.try_wait().ok().flatten().is_some();
    if !exited_before_kill {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map(|status| exit_status_log_fields(&status))
        .unwrap_or_else(|error| format!("wait_error={error:?}"));
    log_offline_render_event(format!(
        "backend=render_server event=server-stopped pid={pid} exited_before_kill={exited_before_kill} {status}"
    ));
}

#[cfg(test)]
mod tests;
