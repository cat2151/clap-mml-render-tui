use std::{io::Cursor, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use clack_host::prelude::PluginEntry;
use cmrt_core::{
    mml_render_with_probe, prepare_cache_render_inputs, render_prepared_cache_with_probe,
    CacheRenderInputs, CoreConfig, NativeRenderProbeContext,
};
use hound::SampleFormat;

use crate::config::{Config, OfflineRenderBackend};
use render_server::RenderServerSupervisor;

#[path = "offline_render/render_server.rs"]
mod render_server;

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
