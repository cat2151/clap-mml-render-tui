//! MML のオフラインレンダリング（notepad / DAW / `render-mml` / `--server` / CLI 共通）。
//!
//! レンダリングは別プロセスの render-server（`POST /render`）だけが行う。この crate も
//! それを使う側の TUI も CLAP plugin をロードしない。
//!
//! グローバルログへの書き込みは app 側の sink 注入（[`set_log_sink`]）で有効になる。
//! 未注入だとログが黙って消えるため、注入は app 起動時に必ず行うこと。

use std::{io::Cursor, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use cmrt_runtime::Config;
use hound::SampleFormat;

use render_server::RenderServerSupervisor;

mod render_server;

pub use cmrt_core::EffectPlugins;

const RENDER_SERVER_PATH: &str = "/render";
const RENDER_SERVER_PATCH_NAME: &str = "(render-server)";
const RENDER_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const RENDER_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const RENDER_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct OfflineRenderer {
    supervisor: Arc<RenderServerSupervisor>,
}

pub struct OfflineRenderOutput {
    pub samples: Vec<f32>,
    pub patch_name: String,
}

/// [`OfflineRenderer::prepare_cache_render`] が返す、レンダリング待ちの MML。
pub struct PreparedOfflineRender(String);

impl OfflineRenderer {
    pub fn new(cfg: Arc<Config>) -> Self {
        Self {
            supervisor: Arc::new(RenderServerSupervisor::new(&cfg)),
        }
    }

    pub fn render_phrase(&self, mml: &str) -> Result<OfflineRenderOutput> {
        self.supervisor.render_mml(mml)
    }

    pub fn prepare_cache_render(&self, mml: &str) -> Result<PreparedOfflineRender> {
        Ok(PreparedOfflineRender(mml.to_string()))
    }

    pub fn render_prepared_cache(&self, prepared: PreparedOfflineRender) -> Result<Vec<f32>> {
        self.supervisor
            .render_mml(&prepared.0)
            .map(|rendered| rendered.samples)
    }
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

type LogSink = fn(&str);
static LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合、この crate のログは黙って捨てられる。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

fn log_offline_render_event(message: impl Into<String>) {
    if let Some(sink) = LOG_SINK.get() {
        sink(&format!("offline-render: {}", message.into()));
    }
}

#[cfg(test)]
mod tests;
