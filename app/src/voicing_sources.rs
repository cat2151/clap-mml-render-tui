//! keyboard用の共有voicing判定を取得し、設定ディレクトリへ永続化する。

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{config::Config, history::VoicingCache, realtime_play::PatchVoicing};

const MAX_SOURCE_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Clone, Debug)]
struct SourceSpec {
    label: &'static str,
    source: String,
    data_path: PathBuf,
    metadata_path: PathBuf,
    config_dir: PathBuf,
}

impl SourceSpec {
    fn enabled(&self) -> bool {
        !self.source.trim().is_empty()
    }

    fn is_url(&self) -> bool {
        let source = self.source.trim();
        source.starts_with("https://") || source.starts_with("http://")
    }

    fn local_source_path(&self) -> PathBuf {
        let path = PathBuf::from(self.source.trim());
        if path.is_absolute() {
            path
        } else {
            self.config_dir.join(path)
        }
    }
}

#[derive(Clone, Debug)]
struct SourceSet {
    shared: SourceSpec,
    override_: SourceSpec,
}

impl SourceSet {
    fn from_config(cfg: &Config) -> Option<Self> {
        let config_dir = crate::config::config_app_dir()?;
        Some(Self::new(
            &config_dir,
            cfg.voicing_shared_source.clone(),
            cfg.voicing_override_source.clone(),
        ))
    }

    fn new(config_dir: &Path, shared_source: String, override_source: String) -> Self {
        let voicing_dir = config_dir.join("voicing");
        Self {
            shared: SourceSpec {
                label: "shared",
                source: shared_source,
                data_path: voicing_dir.join("shared.json"),
                metadata_path: voicing_dir.join("shared-http-metadata.json"),
                config_dir: config_dir.to_path_buf(),
            },
            override_: SourceSpec {
                label: "override",
                source: override_source,
                data_path: voicing_dir.join("override.json"),
                metadata_path: voicing_dir.join("override-http-metadata.json"),
                config_dir: config_dir.to_path_buf(),
            },
        }
    }

    fn specs_missing_persistent_copy(&self) -> bool {
        [&self.shared, &self.override_]
            .into_iter()
            .any(|spec| spec.enabled() && !spec.data_path.exists())
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct VoicingLayers {
    shared: VoicingCache,
    override_: VoicingCache,
}

impl VoicingLayers {
    pub(crate) fn resolve(&self, user: &VoicingCache, patch: &str) -> Option<PatchVoicing> {
        self.override_
            .get(patch)
            .or_else(|| user.get(patch))
            .or_else(|| self.shared.get(patch))
    }
}

#[derive(Clone)]
pub(crate) struct VoicingSourceRefresh {
    sources: Option<SourceSet>,
    completion: Arc<(Mutex<bool>, Condvar)>,
    io_lock: Arc<Mutex<()>>,
}

impl VoicingSourceRefresh {
    pub(crate) fn spawn(cfg: &Config) -> Self {
        let refresh = Self {
            sources: SourceSet::from_config(cfg),
            completion: Arc::new((Mutex::new(false), Condvar::new())),
            io_lock: Arc::new(Mutex::new(())),
        };
        refresh.spawn_worker();
        refresh
    }

    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            sources: None,
            completion: Arc::new((Mutex::new(true), Condvar::new())),
            io_lock: Arc::new(Mutex::new(())),
        }
    }

    fn spawn_worker(&self) {
        let sources = self.sources.clone();
        let completion = Arc::clone(&self.completion);
        let io_lock = Arc::clone(&self.io_lock);
        std::thread::spawn(move || {
            if let Some(sources) = sources {
                refresh_source_set(&sources, &io_lock, || completion.1.notify_all());
            }
            let (completed, signal) = &*completion;
            *completed.lock().unwrap() = true;
            signal.notify_all();
        });
    }

    pub(crate) fn load_for_keyboard(&self) -> VoicingLayers {
        let Some(sources) = &self.sources else {
            return VoicingLayers::default();
        };
        if sources.specs_missing_persistent_copy() {
            let (completed, signal) = &*self.completion;
            let mut completed = completed.lock().unwrap();
            while sources.specs_missing_persistent_copy() && !*completed {
                completed = signal.wait(completed).unwrap();
            }
        }
        let _guard = self.io_lock.lock().unwrap();
        VoicingLayers {
            shared: load_persisted_cache(&sources.shared),
            override_: load_persisted_cache(&sources.override_),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct HttpMetadata {
    source: String,
    #[serde(default)]
    etag: Option<String>,
    #[serde(default)]
    last_modified: Option<String>,
}

fn refresh_source_set(sources: &SourceSet, io_lock: &Mutex<()>, mut notify_progress: impl FnMut()) {
    let mut specs = [&sources.shared, &sources.override_];
    specs.sort_by_key(|spec| spec.data_path.exists());
    for spec in specs {
        if !spec.enabled() {
            continue;
        }
        if let Err(error) = refresh_source(spec, io_lock) {
            log_event(format!(
                "event=refresh-failed source={} error={error:#}",
                spec.label
            ));
        }
        notify_progress();
    }
}

fn refresh_source(spec: &SourceSpec, io_lock: &Mutex<()>) -> Result<()> {
    if spec.is_url() {
        refresh_url_source(spec, io_lock)
    } else {
        refresh_local_source(spec, io_lock)
    }
}

fn refresh_local_source(spec: &SourceSpec, io_lock: &Mutex<()>) -> Result<()> {
    let source_path = spec.local_source_path();
    let bytes = fs::read(&source_path)
        .with_context(|| format!("ローカルsourceを読めません: {}", source_path.display()))?;
    validate_source_bytes(&bytes, spec.label)?;
    let _guard = io_lock.lock().unwrap();
    write_if_changed(&spec.data_path, &bytes)?;
    Ok(())
}

fn refresh_url_source(spec: &SourceSpec, io_lock: &Mutex<()>) -> Result<()> {
    let metadata = load_http_metadata(&spec.metadata_path);
    let can_revalidate = spec.data_path.exists() && metadata.source == spec.source.trim();
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .build();
    let mut request = agent.get(spec.source.trim());
    if can_revalidate {
        if let Some(etag) = metadata.etag.as_deref() {
            request = request.set("If-None-Match", etag);
        }
        if let Some(last_modified) = metadata.last_modified.as_deref() {
            request = request.set("If-Modified-Since", last_modified);
        }
    }

    match request.call() {
        Ok(response) if response.status() == 304 && can_revalidate => {
            let refreshed = refreshed_metadata(spec, &response, metadata);
            let _guard = io_lock.lock().unwrap();
            write_metadata_if_changed(&spec.metadata_path, &refreshed)
        }
        Ok(response) => store_url_response(spec, response, io_lock),
        Err(ureq::Error::Status(304, response)) if can_revalidate => {
            let refreshed = refreshed_metadata(spec, &response, metadata);
            let _guard = io_lock.lock().unwrap();
            write_metadata_if_changed(&spec.metadata_path, &refreshed)
        }
        Err(error) => Err(anyhow::anyhow!("URL取得に失敗しました: {error}")),
    }
}

fn store_url_response(
    spec: &SourceSpec,
    response: ureq::Response,
    io_lock: &Mutex<()>,
) -> Result<()> {
    let metadata = metadata_from_response(spec, &response);
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        anyhow::bail!(
            "source JSONが上限の{} bytesを超えています",
            MAX_SOURCE_BYTES
        );
    }
    validate_source_bytes(&bytes, spec.label)?;
    let _guard = io_lock.lock().unwrap();
    write_if_changed(&spec.data_path, &bytes)?;
    write_metadata_if_changed(&spec.metadata_path, &metadata)
}

fn metadata_from_response(spec: &SourceSpec, response: &ureq::Response) -> HttpMetadata {
    HttpMetadata {
        source: spec.source.trim().to_string(),
        etag: response.header("ETag").map(str::to_string),
        last_modified: response.header("Last-Modified").map(str::to_string),
    }
}

fn refreshed_metadata(
    spec: &SourceSpec,
    response: &ureq::Response,
    previous: HttpMetadata,
) -> HttpMetadata {
    let current = metadata_from_response(spec, response);
    HttpMetadata {
        source: current.source,
        etag: current.etag.or(previous.etag),
        last_modified: current.last_modified.or(previous.last_modified),
    }
}

fn load_http_metadata(path: &Path) -> HttpMetadata {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn validate_source_bytes(bytes: &[u8], label: &str) -> Result<()> {
    let text =
        std::str::from_utf8(bytes).with_context(|| format!("{label} JSONがUTF-8ではありません"))?;
    VoicingCache::from_shared_json(text)
        .with_context(|| format!("{label} JSONの検証に失敗しました"))?;
    Ok(())
}

fn load_persisted_cache(spec: &SourceSpec) -> VoicingCache {
    if !spec.enabled() {
        return VoicingCache::default();
    }
    let result = fs::read_to_string(&spec.data_path)
        .with_context(|| format!("永続JSONを読めません: {}", spec.data_path.display()))
        .and_then(|text| VoicingCache::from_shared_json(&text));
    match result {
        Ok(cache) => cache,
        Err(error) => {
            log_event(format!(
                "event=load-failed source={} error={error:#}",
                spec.label
            ));
            VoicingCache::default()
        }
    }
}

fn write_metadata_if_changed(path: &Path, metadata: &HttpMetadata) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(metadata)?;
    write_if_changed(path, &bytes)?;
    Ok(())
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(true)
}

fn log_event(message: String) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("voicing-source: {message}"));
    #[cfg(test)]
    let _ = message;
}

#[cfg(test)]
#[path = "tests/voicing_sources.rs"]
mod tests;
