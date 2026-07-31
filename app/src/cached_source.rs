//! HTTP(S) URL またはローカルファイルを source として、設定ディレクトリ配下へ
//! JSON をキャッシュする共通処理。
//!
//! keyboard の mono/poly 判定（`voicing_sources`）と grid sequencer のコード進行
//! カタログ（`chord_progression_source`）が共有する。URL の場合は ETag /
//! Last-Modified による条件付き GET を行い、304 のときは本体を読み直さない。

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// 1 source あたりの受け入れ上限。壊れた/巨大なレスポンスでディスクを埋めない。
const MAX_SOURCE_BYTES: u64 = 5 * 1024 * 1024;

/// 保存前に「内容が期待する JSON か」を確かめる関数。
/// 検証に落ちた取得結果は捨て、既存のキャッシュを残す。
pub(crate) type SourceValidator = fn(&[u8]) -> Result<()>;

/// 取得の結果、キャッシュの中身が変わったかどうか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceRefreshOutcome {
    Updated,
    Unchanged,
}

/// source（URL またはパス）と、そのローカルキャッシュ先の組。
#[derive(Clone, Debug)]
pub(crate) struct CachedSource {
    pub(crate) label: &'static str,
    pub(crate) source: String,
    pub(crate) data_path: PathBuf,
    pub(crate) metadata_path: PathBuf,
    config_dir: PathBuf,
}

impl CachedSource {
    /// `relative_data_path` は設定ディレクトリからの相対パス（例: `voicing/shared.json`）。
    /// HTTP メタデータは同じディレクトリの `<stem>-http-metadata.json` へ置く。
    pub(crate) fn new(
        label: &'static str,
        config_dir: &Path,
        source: String,
        relative_data_path: &str,
    ) -> Self {
        let data_path = config_dir.join(relative_data_path);
        let stem = data_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| label.to_string());
        let metadata_path = data_path.with_file_name(format!("{stem}-http-metadata.json"));
        Self {
            label,
            source,
            data_path,
            metadata_path,
            config_dir: config_dir.to_path_buf(),
        }
    }

    /// source が空文字なら、その source は無効（取得も読み込みもしない）。
    pub(crate) fn enabled(&self) -> bool {
        !self.source.trim().is_empty()
    }

    pub(crate) fn is_url(&self) -> bool {
        let source = self.source.trim();
        source.starts_with("https://") || source.starts_with("http://")
    }

    /// 有効なのにローカルキャッシュがまだ無い状態か。初回だけ取得完了を待つ判定に使う。
    pub(crate) fn missing_cache(&self) -> bool {
        self.enabled() && !self.data_path.exists()
    }

    pub(crate) fn read_cached(&self) -> Result<String> {
        fs::read_to_string(&self.data_path)
            .with_context(|| format!("永続JSONを読めません: {}", self.data_path.display()))
    }

    fn local_source_path(&self) -> PathBuf {
        let path = PathBuf::from(self.source.trim());
        if path.is_absolute() {
            path
        } else {
            self.config_dir.join(path)
        }
    }

    /// source を取り直してキャッシュを更新する。
    pub(crate) fn refresh(
        &self,
        io_lock: &Mutex<()>,
        validate: SourceValidator,
    ) -> Result<SourceRefreshOutcome> {
        if self.is_url() {
            self.refresh_from_url(io_lock, validate)
        } else {
            self.refresh_from_local(io_lock, validate)
        }
    }

    fn refresh_from_local(
        &self,
        io_lock: &Mutex<()>,
        validate: SourceValidator,
    ) -> Result<SourceRefreshOutcome> {
        let source_path = self.local_source_path();
        let bytes = fs::read(&source_path)
            .with_context(|| format!("ローカルsourceを読めません: {}", source_path.display()))?;
        self.validate_bytes(&bytes, validate)?;
        let _guard = io_lock.lock().unwrap();
        Ok(outcome(write_if_changed(&self.data_path, &bytes)?))
    }

    fn refresh_from_url(
        &self,
        io_lock: &Mutex<()>,
        validate: SourceValidator,
    ) -> Result<SourceRefreshOutcome> {
        let metadata = load_http_metadata(&self.metadata_path);
        let can_revalidate = self.data_path.exists() && metadata.source == self.source.trim();
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(10))
            .build();
        let mut request = agent.get(self.source.trim());
        if can_revalidate {
            if let Some(etag) = metadata.etag.as_deref() {
                request = request.set("If-None-Match", etag);
            }
            if let Some(last_modified) = metadata.last_modified.as_deref() {
                request = request.set("If-Modified-Since", last_modified);
            }
        }

        // ureq は 304 を Ok で返すことも Err(Status) で返すこともあるため両方を受ける。
        match request.call() {
            Ok(response) if response.status() == 304 && can_revalidate => {
                self.store_not_modified(&response, metadata, io_lock)
            }
            Ok(response) => self.store_response(response, io_lock, validate),
            Err(ureq::Error::Status(304, response)) if can_revalidate => {
                self.store_not_modified(&response, metadata, io_lock)
            }
            Err(error) => Err(anyhow::anyhow!("URL取得に失敗しました: {error}")),
        }
    }

    fn store_not_modified(
        &self,
        response: &ureq::Response,
        previous: HttpMetadata,
        io_lock: &Mutex<()>,
    ) -> Result<SourceRefreshOutcome> {
        let refreshed = self.refreshed_metadata(response, previous);
        let _guard = io_lock.lock().unwrap();
        write_metadata_if_changed(&self.metadata_path, &refreshed)?;
        Ok(SourceRefreshOutcome::Unchanged)
    }

    fn store_response(
        &self,
        response: ureq::Response,
        io_lock: &Mutex<()>,
        validate: SourceValidator,
    ) -> Result<SourceRefreshOutcome> {
        let metadata = self.metadata_from_response(&response);
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
        self.validate_bytes(&bytes, validate)?;
        let _guard = io_lock.lock().unwrap();
        let changed = write_if_changed(&self.data_path, &bytes)?;
        write_metadata_if_changed(&self.metadata_path, &metadata)?;
        Ok(outcome(changed))
    }

    fn validate_bytes(&self, bytes: &[u8], validate: SourceValidator) -> Result<()> {
        std::str::from_utf8(bytes)
            .with_context(|| format!("{} JSONがUTF-8ではありません", self.label))?;
        validate(bytes).with_context(|| format!("{} JSONの検証に失敗しました", self.label))
    }

    fn metadata_from_response(&self, response: &ureq::Response) -> HttpMetadata {
        HttpMetadata {
            source: self.source.trim().to_string(),
            etag: response.header("ETag").map(str::to_string),
            last_modified: response.header("Last-Modified").map(str::to_string),
        }
    }

    fn refreshed_metadata(
        &self,
        response: &ureq::Response,
        previous: HttpMetadata,
    ) -> HttpMetadata {
        let current = self.metadata_from_response(response);
        HttpMetadata {
            source: current.source,
            etag: current.etag.or(previous.etag),
            last_modified: current.last_modified.or(previous.last_modified),
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

fn outcome(changed: bool) -> SourceRefreshOutcome {
    if changed {
        SourceRefreshOutcome::Updated
    } else {
        SourceRefreshOutcome::Unchanged
    }
}

fn load_http_metadata(path: &Path) -> HttpMetadata {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_metadata_if_changed(path: &Path, metadata: &HttpMetadata) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(metadata)?;
    write_if_changed(path, &bytes)?;
    Ok(())
}

/// 内容が変わったときだけ書き、実際に書いたかどうかを返す。
pub(crate) fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
