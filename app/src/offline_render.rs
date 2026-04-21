use std::{
    io::{Cursor, Read as _},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _, Result};
use clack_host::prelude::PluginEntry;
use cmrt_core::{
    mml_render_with_probe, prepare_cache_render_inputs, render_prepared_cache_with_probe,
    CacheRenderInputs, CoreConfig, NativeRenderProbeContext,
};
use hound::SampleFormat;

use crate::config::{Config, OfflineRenderBackend};

const RENDER_SERVER_PATH: &str = "/render";
const RENDER_SERVER_PATCH_NAME: &str = "(render-server)";
const RENDER_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const RENDER_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const RENDER_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub(crate) struct OfflineRenderer {
    backend: Arc<OfflineRendererBackend>,
}

pub(crate) struct OfflineRenderOutput {
    pub(crate) samples: Vec<f32>,
    pub(crate) patch_name: String,
}

pub(crate) enum PreparedOfflineRender {
    InProcess(CacheRenderInputs),
    RenderServer(String),
}

enum OfflineRendererBackend {
    InProcess { cfg: Arc<Config>, entry_ptr: usize },
    RenderServer { supervisor: RenderServerSupervisor },
}

impl OfflineRenderer {
    pub(crate) fn new(cfg: Arc<Config>, entry_ptr: usize) -> Self {
        let backend = match cfg.offline_render_backend {
            OfflineRenderBackend::InProcess => OfflineRendererBackend::InProcess { cfg, entry_ptr },
            OfflineRenderBackend::RenderServer => OfflineRendererBackend::RenderServer {
                supervisor: RenderServerSupervisor::new(&cfg),
            },
        };
        Self {
            backend: Arc::new(backend),
        }
    }

    pub(crate) fn render_phrase(
        &self,
        mml: &str,
        probe_context: Option<&NativeRenderProbeContext>,
    ) -> Result<OfflineRenderOutput> {
        match self.backend.as_ref() {
            OfflineRendererBackend::InProcess { cfg, entry_ptr } => {
                let entry = plugin_entry(*entry_ptr)?;
                let core_cfg = CoreConfig::from(cfg.as_ref());
                let (samples, patch_name) =
                    mml_render_with_probe(mml, &core_cfg, entry, probe_context)?;
                Ok(OfflineRenderOutput {
                    samples,
                    patch_name,
                })
            }
            OfflineRendererBackend::RenderServer { supervisor } => supervisor.render_mml(mml),
        }
    }

    pub(crate) fn prepare_cache_render(&self, mml: &str) -> Result<PreparedOfflineRender> {
        match self.backend.as_ref() {
            OfflineRendererBackend::InProcess { cfg, .. } => {
                let core_cfg = CoreConfig::from(cfg.as_ref());
                prepare_cache_render_inputs(mml, &core_cfg).map(PreparedOfflineRender::InProcess)
            }
            OfflineRendererBackend::RenderServer { .. } => {
                Ok(PreparedOfflineRender::RenderServer(mml.to_string()))
            }
        }
    }

    pub(crate) fn render_prepared_cache(
        &self,
        prepared: PreparedOfflineRender,
        probe_context: Option<&NativeRenderProbeContext>,
    ) -> Result<Vec<f32>> {
        match (self.backend.as_ref(), prepared) {
            (
                OfflineRendererBackend::InProcess { entry_ptr, .. },
                PreparedOfflineRender::InProcess(prepared),
            ) => {
                let entry = plugin_entry(*entry_ptr)?;
                render_prepared_cache_with_probe(prepared, entry, probe_context)
            }
            (
                OfflineRendererBackend::RenderServer { supervisor },
                PreparedOfflineRender::RenderServer(mml),
            ) => supervisor.render_mml(&mml).map(|rendered| rendered.samples),
            (OfflineRendererBackend::InProcess { .. }, PreparedOfflineRender::RenderServer(_))
            | (OfflineRendererBackend::RenderServer { .. }, PreparedOfflineRender::InProcess(_)) => {
                Err(anyhow!(
                    "offline render backend changed while a render job was prepared"
                ))
            }
        }
    }
}

fn plugin_entry(entry_ptr: usize) -> Result<&'static PluginEntry> {
    if entry_ptr == 0 {
        anyhow::bail!("in-process offline render requires a loaded CLAP PluginEntry");
    }
    // SAFETY: production callers pass a pointer to the PluginEntry owned by main(), and
    // existing render workers already rely on that entry outliving the worker threads.
    Ok(unsafe { &*(entry_ptr as *const PluginEntry) })
}

struct RenderServerSupervisor {
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
    fn new(cfg: &Config) -> Self {
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

    fn render_mml(&self, mml: &str) -> Result<OfflineRenderOutput> {
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

fn decode_wav_bytes(bytes: &[u8], expected_sample_rate: u32) -> Result<Vec<f32>> {
    let cursor = Cursor::new(bytes);
    let mut reader =
        hound::WavReader::new(cursor).map_err(|error| anyhow!("WAV decode failed: {error}"))?;
    let spec = reader.spec();
    if spec.channels != 2 {
        anyhow::bail!(
            "render-server returned {}ch WAV; expected stereo",
            spec.channels
        );
    }
    if spec.sample_rate != expected_sample_rate {
        anyhow::bail!(
            "render-server returned {}Hz WAV; expected {}Hz",
            spec.sample_rate,
            expected_sample_rate
        );
    }

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?,
        _ => anyhow::bail!(
            "render-server returned unsupported WAV format: {:?} {}bit",
            spec.sample_format,
            spec.bits_per_sample
        ),
    };
    if samples.len() % 2 != 0 {
        anyhow::bail!("render-server returned malformed stereo WAV sample count");
    }
    Ok(samples)
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

fn log_offline_render_event(message: impl Into<String>) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("offline-render: {}", message.into()));
    #[cfg(test)]
    let _ = message.into();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_bytes_i16(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let cursor = Cursor::new(&mut bytes);
            let spec = hound::WavSpec {
                channels,
                sample_rate,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::new(cursor, spec).unwrap();
            for sample in samples {
                writer.write_sample(*sample).unwrap();
            }
            writer.finalize().unwrap();
        }
        bytes
    }

    #[test]
    fn decode_wav_bytes_accepts_16bit_stereo() {
        let bytes = wav_bytes_i16(48_000, 2, &[0, i16::MAX, i16::MIN, 0]);

        let samples = decode_wav_bytes(&bytes, 48_000).unwrap();

        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0], 0.0);
        assert!((samples[1] - 1.0).abs() < f32::EPSILON);
        assert!(samples[2] <= -1.0);
    }

    #[test]
    fn decode_wav_bytes_rejects_unexpected_sample_rate() {
        let bytes = wav_bytes_i16(44_100, 2, &[0, 0]);

        let error = decode_wav_bytes(&bytes, 48_000).unwrap_err();

        assert!(error.to_string().contains("expected 48000Hz"));
    }
}
