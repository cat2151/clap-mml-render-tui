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

    pub fn get_patches(&self) -> Result<Vec<String>, Error> {
        let response = self
            .agent
            .get(&self.endpoint_url("/patches"))
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
        let status: StatusResponse = response
            .into_json()
            .map_err(|error| Error::InvalidResponse(error.to_string()))?;
        if status.status == "ok" {
            Ok(())
        } else {
            Err(Error::Http {
                status: 200,
                body: format!("unexpected status response: {}", status.status),
            })
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
mod tests {
    use super::{DawClient, Error, DEFAULT_BASE_URL};

    #[test]
    fn local_default_uses_known_base_url() {
        let client = DawClient::local_default();

        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn new_trims_whitespace_and_trailing_slashes() {
        let client = DawClient::new(" http://127.0.0.1:62151/// ").unwrap();

        assert_eq!(client.base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn new_rejects_empty_base_url() {
        let error = DawClient::new("   ").unwrap_err();

        assert!(matches!(error, Error::EmptyBaseUrl));
    }
}
