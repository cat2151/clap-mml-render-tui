//! MML のオフラインレンダリング（notepad / DAW 共通）。
//!
//! in-process の CLAP ホスト実行と、別プロセスの render server 実行の2バックエンドを
//! 同じインターフェースの背後に隠す。
//!
//! グローバルログへの書き込みは app 側の sink 注入（[`set_log_sink`]）で有効になる。
//! 未注入だとログが黙って消えるため、注入は app 起動時に必ず行うこと。

use std::{io::Cursor, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use cmrt_core::{
    mml_render_with_probe, prepare_cache_render_inputs, render_prepared_cache_with_probe,
    CacheRenderInputs, NativeRenderProbeContext,
};
use cmrt_runtime::{Config, OfflineRenderBackend};
use hound::SampleFormat;

use render_server::RenderServerSupervisor;

mod in_process;
mod plugin_entries;
mod render_server;

pub use in_process::InProcessPlugins;
pub use plugin_entries::PluginEntries;

const RENDER_SERVER_PATH: &str = "/render";
const RENDER_SERVER_PATCH_NAME: &str = "(render-server)";
const RENDER_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const RENDER_SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const RENDER_SERVER_START_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct OfflineRenderer {
    backend: Arc<OfflineRendererBackend>,
}

pub struct OfflineRenderOutput {
    pub samples: Vec<f32>,
    pub patch_name: String,
}

pub enum PreparedOfflineRender {
    /// `plugin` はカタログ上の添字。**prepare 時に決めたプラグインでレンダリングする。**
    /// ここで持たずにレンダー時へ引き直すと、prepare 済み MML（先頭 JSON が解決済みの
    /// 絶対パスへ書き換わっている）から形を読み直すことになり、判別材料が変わる。
    InProcess {
        prepared: CacheRenderInputs,
        plugin: usize,
    },
    RenderServer(String),
}

enum OfflineRendererBackend {
    InProcess(InProcessPlugins),
    RenderServer { supervisor: RenderServerSupervisor },
}

impl OfflineRenderer {
    pub fn new(cfg: Arc<Config>, entries: PluginEntries) -> Self {
        let backend = match cfg.offline_render_backend {
            OfflineRenderBackend::InProcess => {
                OfflineRendererBackend::InProcess(InProcessPlugins::new(cfg.as_ref(), &entries))
            }
            OfflineRenderBackend::RenderServer => OfflineRendererBackend::RenderServer {
                supervisor: RenderServerSupervisor::new(&cfg),
            },
        };
        Self {
            backend: Arc::new(backend),
        }
    }

    pub fn render_phrase(
        &self,
        mml: &str,
        probe_context: Option<&NativeRenderProbeContext>,
    ) -> Result<OfflineRenderOutput> {
        match self.backend.as_ref() {
            OfflineRendererBackend::InProcess(plugins) => {
                let (entry, core_cfg) = plugins.for_mml(mml)?;
                let (samples, patch_name) =
                    mml_render_with_probe(mml, core_cfg, entry, probe_context)?;
                Ok(OfflineRenderOutput {
                    samples,
                    patch_name,
                })
            }
            OfflineRendererBackend::RenderServer { supervisor } => supervisor.render_mml(mml),
        }
    }

    pub fn prepare_cache_render(&self, mml: &str) -> Result<PreparedOfflineRender> {
        match self.backend.as_ref() {
            OfflineRendererBackend::InProcess(plugins) => {
                let plugin = plugins.index_for_mml(mml);
                prepare_cache_render_inputs(mml, plugins.core_cfg(plugin))
                    .map(|prepared| PreparedOfflineRender::InProcess { prepared, plugin })
            }
            OfflineRendererBackend::RenderServer { .. } => {
                Ok(PreparedOfflineRender::RenderServer(mml.to_string()))
            }
        }
    }

    pub fn render_prepared_cache(
        &self,
        prepared: PreparedOfflineRender,
        probe_context: Option<&NativeRenderProbeContext>,
    ) -> Result<Vec<f32>> {
        match (self.backend.as_ref(), prepared) {
            (
                OfflineRendererBackend::InProcess(plugins),
                PreparedOfflineRender::InProcess { prepared, plugin },
            ) => render_prepared_cache_with_probe(prepared, plugins.entry(plugin)?, probe_context),
            (
                OfflineRendererBackend::RenderServer { supervisor },
                PreparedOfflineRender::RenderServer(mml),
            ) => supervisor.render_mml(&mml).map(|rendered| rendered.samples),
            (OfflineRendererBackend::InProcess(_), PreparedOfflineRender::RenderServer(_))
            | (
                OfflineRendererBackend::RenderServer { .. },
                PreparedOfflineRender::InProcess { .. },
            ) => Err(anyhow!(
                "offline render backend changed while a render job was prepared"
            )),
        }
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
