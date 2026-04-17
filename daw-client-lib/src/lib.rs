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
struct PostAbRepeatRequest {
    #[serde(rename = "measA")]
    start_measure: usize,
    #[serde(rename = "measB")]
    end_measure: usize,
}

impl DawClient {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, Error> {
        let base_url = normalize_base_url(base_url.as_ref())?;
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout_read(READ_WRITE_TIMEOUT)
            .timeout_write(READ_WRITE_TIMEOUT)
            .build();
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

    pub fn get_mml(&self, track: usize, measure: usize) -> Result<String, Error> {
        let response: GetMmlResponse =
            self.get_json(&format!("/mml?track={track}&measure={measure}"))?;
        Ok(response.mml)
    }

    pub fn get_mmls(&self, if_none_match: Option<&str>) -> Result<Option<GetMmlsResponse>, Error> {
        let mut request = self.agent.get(&self.endpoint_url("/mmls"));
        if let Some(etag) = if_none_match {
            request = request.set("If-None-Match", etag);
        }
        match request.call() {
            Ok(response) if response.status() == 304 => Ok(None),
            Ok(response) => {
                let etag = response
                    .header("ETag")
                    .ok_or_else(|| Error::InvalidResponse("missing ETag header".to_string()))?
                    .to_string();
                let body: GetMmlsBody = response
                    .into_json()
                    .map_err(|error| Error::InvalidResponse(error.to_string()))?;
                Ok(Some(GetMmlsResponse {
                    etag,
                    tracks: body.tracks,
                }))
            }
            Err(error) => Err(Error::from_ureq(error)),
        }
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let response = self
            .agent
            .get(&self.endpoint_url(path))
            .call()
            .map_err(Error::from_ureq)?;
        response
            .into_json()
            .map_err(|error| Error::InvalidResponse(error.to_string()))
    }

    fn post_status<T: Serialize>(&self, path: &str, body: T) -> Result<(), Error> {
        let response = self
            .agent
            .post(&self.endpoint_url(path))
            .send_json(body)
            .map_err(Error::from_ureq)?;
        self.read_status_response(response)
    }

    fn post_empty_status(&self, path: &str) -> Result<(), Error> {
        let response = self
            .agent
            .post(&self.endpoint_url(path))
            .call()
            .map_err(Error::from_ureq)?;
        self.read_status_response(response)
    }

    fn read_status_response(&self, response: ureq::Response) -> Result<(), Error> {
        let http_status = response.status();
        let status: StatusResponse = response
            .into_json()
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
            ureq::Error::Status(status, response) => {
                let body = response.into_string().unwrap_or_default();
                Self::Http { status, body }
            }
            ureq::Error::Transport(error) => Self::Transport(error.to_string()),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBaseUrl => write!(f, "base url must not be empty"),
            Self::Http { status, body } => {
                write!(f, "http request failed with status {status}: {body}")
            }
            Self::Transport(error) => write!(f, "http transport error: {error}"),
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
#[path = "tests.rs"]
mod tests;
