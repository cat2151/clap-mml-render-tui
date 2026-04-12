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
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
        time::Duration,
    };

    use super::{DawClient, Error, DEFAULT_BASE_URL};

    fn spawn_single_request_server(response_body: &str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let (request_tx, request_rx) = mpsc::channel();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let request = read_http_request(&mut stream);
            request_tx.send(request).unwrap();
            stream.write_all(response.as_bytes()).unwrap();
        });

        (address, request_rx)
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let header_end = loop {
            let read = stream.read(&mut buffer).unwrap();
            assert!(read > 0, "request closed before headers were complete");
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };

        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("Content-Length") {
                    Some(value.trim().parse::<usize>().unwrap())
                } else {
                    None
                }
            })
            .unwrap_or(0);

        while bytes.len() < header_end + content_length {
            let read = stream.read(&mut buffer).unwrap();
            assert!(read > 0, "request closed before body was complete");
            bytes.extend_from_slice(&buffer[..read]);
        }

        String::from_utf8(bytes).unwrap()
    }

    fn request_body(request: &str) -> &str {
        request.split_once("\r\n\r\n").unwrap().1
    }

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

    #[test]
    fn post_mml_sends_expected_request() {
        let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
        let client = DawClient::new(&base_url).unwrap();

        client.post_mml(2, 3, "l8cde").unwrap();

        let request = request_rx.recv().unwrap();
        assert!(request.starts_with("POST /mml HTTP/1.1\r\n"));
        assert_eq!(
            request_body(&request),
            r#"{"track":2,"measure":3,"mml":"l8cde"}"#
        );
    }

    #[test]
    fn post_mixer_sends_expected_request() {
        let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
        let client = DawClient::new(&base_url).unwrap();

        client.post_mixer(4, -6.5).unwrap();

        let request = request_rx.recv().unwrap();
        assert!(request.starts_with("POST /mixer HTTP/1.1\r\n"));
        assert_eq!(request_body(&request), r#"{"track":4,"db":-6.5}"#);
    }

    #[test]
    fn post_patch_sends_expected_request() {
        let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
        let client = DawClient::new(&base_url).unwrap();

        client.post_patch(1, "Pads/Factory Pad.fxp").unwrap();

        let request = request_rx.recv().unwrap();
        assert!(request.starts_with("POST /patch HTTP/1.1\r\n"));
        assert_eq!(
            request_body(&request),
            r#"{"track":1,"patch":"Pads/Factory Pad.fxp"}"#
        );
    }

    #[test]
    fn get_patches_reads_json_response() {
        let (base_url, request_rx) =
            spawn_single_request_server(r#"["Pads/Factory Pad.fxp","Lead/Bright.fxp"]"#);
        let client = DawClient::new(&base_url).unwrap();

        let patches = client.get_patches().unwrap();

        let request = request_rx.recv().unwrap();
        assert!(request.starts_with("GET /patches HTTP/1.1\r\n"));
        assert_eq!(
            patches,
            vec![
                "Pads/Factory Pad.fxp".to_string(),
                "Lead/Bright.fxp".to_string()
            ]
        );
    }
}
