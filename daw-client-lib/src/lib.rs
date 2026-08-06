use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:62151";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct DawClient {
    agent: ureq::Agent,
    base_url: String,
}

#[derive(Debug)]
pub enum Error {
    EmptyBaseUrl,
    Http { status: u16, body: String },
    Transport(String),
    InvalidRequest(String),
    InvalidResponse(String),
}

#[derive(Deserialize)]
struct StatusResponse {
    status: String,
}

#[derive(Deserialize)]
struct GetMmlResponse {
    mml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetMmlsResponse {
    pub etag: String,
    pub tracks: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DawStatusResponse {
    pub mode: String,
    pub play: DawStatusPlay,
    pub cache: DawStatusCache,
    pub grid: DawStatusGrid,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DawStatusPlay {
    pub state: String,
    pub is_playing: bool,
    pub is_preview: bool,
    pub current_measure: Option<usize>,
    pub current_measure_index: Option<usize>,
    pub current_beat: Option<u32>,
    pub measure_elapsed_ms: Option<u64>,
    pub measure_duration_ms: Option<u64>,
    #[serde(rename = "loop")]
    pub loop_status: DawStatusLoop,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DawStatusLoop {
    pub enabled: bool,
    pub start_measure: Option<usize>,
    pub end_measure: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DawStatusCache {
    pub active_render_count: usize,
    pub pending_count: usize,
    pub rendering_count: usize,
    pub ready_count: usize,
    pub error_count: usize,
    pub is_updating: bool,
    pub is_complete: bool,
    pub cells: Vec<Vec<DawStatusCacheCell>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DawStatusCacheCell {
    pub state: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DawStatusGrid {
    pub tracks: usize,
    pub measures: usize,
}

#[derive(Deserialize)]
struct GetMmlsBody {
    tracks: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct PostMmlRequest<'a> {
    track: usize,
    measure: usize,
    mml: &'a str,
}

#[derive(Serialize)]
struct PostMixerRequest {
    track: usize,
    db: f64,
}

#[derive(Serialize)]
struct PostPatchRequest<'a> {
    track: usize,
    patch: &'a str,
}

#[derive(Serialize)]
struct PostRandomPatchRequest {
    track: usize,
}

#[derive(Serialize)]
struct PostAbRepeatRequest {
    #[serde(rename = "measA")]
    start_measure: usize,
    #[serde(rename = "measB")]
    end_measure: usize,
}

impl DawClient {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, Error> {
        let base_url = normalize_base_url(base_url.as_ref())?;
        // http_status_as_error(false): 4xx/5xx も Ok として受け取り、
        // 本文をエラーメッセージへ載せられるようにする（ureq 3 の StatusCode error は本文を持たない）。
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_send_body(Some(READ_WRITE_TIMEOUT))
            .timeout_recv_response(Some(READ_WRITE_TIMEOUT))
            .timeout_recv_body(Some(READ_WRITE_TIMEOUT))
            .http_status_as_error(false)
            .build();
        let agent = ureq::Agent::new_with_config(config);
        Ok(Self { agent, base_url })
    }

    pub fn local_default() -> Self {
        Self::new(DEFAULT_BASE_URL).expect("default base url should be valid")
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn post_mml(&self, track: usize, measure: usize, mml: &str) -> Result<(), Error> {
        self.post_status(
            "/mml",
            PostMmlRequest {
                track,
                measure,
                mml,
            },
        )
    }

    pub fn post_mixer(&self, track: usize, db: f64) -> Result<(), Error> {
        self.post_status("/mixer", PostMixerRequest { track, db })
    }

    pub fn post_patch(&self, track: usize, patch: &str) -> Result<(), Error> {
        self.post_status("/patch", PostPatchRequest { track, patch })
    }

    pub fn post_random_patch(&self, track: usize) -> Result<(), Error> {
        self.post_status("/patch/random", PostRandomPatchRequest { track })
    }

    pub fn post_play_start(&self) -> Result<(), Error> {
        self.post_empty_status("/play/start")
    }

    pub fn post_play_stop(&self) -> Result<(), Error> {
        self.post_empty_status("/play/stop")
    }

    pub fn post_daw_mode(&self) -> Result<(), Error> {
        self.post_empty_status("/mode/daw")
    }

    pub fn post_ab_repeat(&self, start_measure: usize, end_measure: usize) -> Result<(), Error> {
        self.post_status(
            "/ab-repeat",
            PostAbRepeatRequest {
                start_measure,
                end_measure,
            },
        )
    }

    pub fn get_patches(&self) -> Result<Vec<String>, Error> {
        self.get_json("/patches")
    }

    pub fn get_status(&self) -> Result<DawStatusResponse, Error> {
        self.get_json("/status")
    }

    pub fn get_mml(&self, track: usize, measure: usize) -> Result<String, Error> {
        let response: GetMmlResponse =
            self.get_json(&format!("/mml?track={track}&measure={measure}"))?;
        Ok(response.mml)
    }

    pub fn get_mmls(&self, if_none_match: Option<&str>) -> Result<Option<GetMmlsResponse>, Error> {
        let mut request = self.agent.get(self.endpoint_url("/mmls"));
        if let Some(etag) = if_none_match {
            request = request.header("If-None-Match", etag);
        }
        let mut response = request.call().map_err(Error::from_ureq)?;
        if response.status() == ureq::http::StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        if let Some(error) = error_for_status(&mut response) {
            return Err(error);
        }
        let etag = response
            .headers()
            .get("ETag")
            .and_then(|etag| etag.to_str().ok())
            .ok_or_else(|| Error::InvalidResponse("missing ETag header".to_string()))?
            .to_string();
        let body: GetMmlsBody = response
            .body_mut()
            .read_json()
            .map_err(|error| Error::InvalidResponse(error.to_string()))?;
        Ok(Some(GetMmlsResponse {
            etag,
            tracks: body.tracks,
        }))
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let mut response = self
            .agent
            .get(self.endpoint_url(path))
            .call()
            .map_err(Error::from_ureq)?;
        if let Some(error) = error_for_status(&mut response) {
            return Err(error);
        }
        response
            .body_mut()
            .read_json()
            .map_err(|error| Error::InvalidResponse(error.to_string()))
    }

    fn post_status<T: Serialize>(&self, path: &str, body: T) -> Result<(), Error> {
        // ureq の send_json は pretty JSON を送るため、compact な body を自前で作って送る。
        let body = serde_json::to_string(&body)
            .map_err(|error| Error::InvalidRequest(error.to_string()))?;
        let response = self
            .agent
            .post(self.endpoint_url(path))
            .header("Content-Type", "application/json")
            .send(body)
            .map_err(Error::from_ureq)?;
        self.read_status_response(response)
    }

    fn post_empty_status(&self, path: &str) -> Result<(), Error> {
        let response = self
            .agent
            .post(self.endpoint_url(path))
            .send_empty()
            .map_err(Error::from_ureq)?;
        self.read_status_response(response)
    }

    fn read_status_response(
        &self,
        mut response: ureq::http::Response<ureq::Body>,
    ) -> Result<(), Error> {
        if let Some(error) = error_for_status(&mut response) {
            return Err(error);
        }
        let http_status = response.status().as_u16();
        let status: StatusResponse = response
            .body_mut()
            .read_json()
            .map_err(|error| Error::InvalidResponse(error.to_string()))?;
        if status.status == "ok" {
            Ok(())
        } else {
            Err(Error::InvalidResponse(format!(
                "unexpected status response (http {}): {}",
                http_status, status.status
            )))
        }
    }

    fn endpoint_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl Error {
    fn from_ureq(error: ureq::Error) -> Self {
        match error {
            // Agent は http_status_as_error(false) なので通常ここには来ないが、
            // 設定漏れで来ても status を落とさないよう受けておく。
            ureq::Error::StatusCode(status) => Self::Http {
                status,
                body: String::new(),
            },
            other => Self::Transport(other.to_string()),
        }
    }
}

/// 2xx 以外を `Error::Http` に変換する。本文はエラーメッセージへ載せる。
fn error_for_status(response: &mut ureq::http::Response<ureq::Body>) -> Option<Error> {
    if response.status().is_success() {
        return None;
    }
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    Some(Error::Http { status, body })
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBaseUrl => write!(f, "base url must not be empty"),
            Self::Http { status, body } => {
                write!(f, "http request failed with status {status}: {body}")
            }
            Self::Transport(error) => write!(f, "http transport error: {error}"),
            Self::InvalidRequest(error) => write!(f, "invalid request body: {error}"),
            Self::InvalidResponse(error) => write!(f, "invalid response body: {error}"),
        }
    }
}

impl std::error::Error for Error {}

fn normalize_base_url(base_url: &str) -> Result<String, Error> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(Error::EmptyBaseUrl);
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests;
