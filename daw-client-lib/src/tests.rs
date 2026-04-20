use std::{
    io::ErrorKind,
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::{
    DawClient, DawStatusCache, DawStatusCacheCell, DawStatusGrid, DawStatusLoop, DawStatusPlay,
    DawStatusResponse, Error, GetMmlsResponse, DEFAULT_BASE_URL,
};

const TEST_READ_TIMEOUT: Duration = Duration::from_secs(2);
const TEST_READ_DEADLINE: Duration = Duration::from_secs(5);

fn spawn_single_request_server(response_body: &str) -> (String, mpsc::Receiver<String>) {
    spawn_single_request_server_with_response(
        "200 OK",
        &["Content-Type: application/json"],
        response_body,
    )
}

fn spawn_single_request_server_with_response(
    status_line: &str,
    headers: &[&str],
    response_body: &str,
) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (request_tx, request_rx) = mpsc::channel();
    let header_block = headers
        .iter()
        .map(|header| format!("{header}\r\n"))
        .collect::<String>();
    let response = format!(
        "HTTP/1.1 {status_line}\r\n{header_block}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(TEST_READ_TIMEOUT)).unwrap();
        let request = read_http_request(&mut stream);
        request_tx.send(request).unwrap();
        stream.write_all(response.as_bytes()).unwrap();
    });

    (address, request_rx)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let deadline = Instant::now() + TEST_READ_DEADLINE;
    let header_end = loop {
        let read = read_with_deadline(stream, &mut buffer, deadline);
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
        let read = read_with_deadline(stream, &mut buffer, deadline);
        assert!(read > 0, "request closed before body was complete");
        bytes.extend_from_slice(&buffer[..read]);
    }

    String::from_utf8(bytes).unwrap()
}

fn read_with_deadline(
    stream: &mut std::net::TcpStream,
    buffer: &mut [u8],
    deadline: Instant,
) -> usize {
    loop {
        match stream.read(buffer) {
            Ok(read) => return read,
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                panic!("timed out while reading HTTP request: {error}");
            }
            Err(error) => panic!("failed to read HTTP request: {error}"),
        }
    }
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
fn post_random_patch_sends_expected_request() {
    let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
    let client = DawClient::new(&base_url).unwrap();

    client.post_random_patch(1).unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("POST /patch/random HTTP/1.1\r\n"));
    assert_eq!(request_body(&request), r#"{"track":1}"#);
}

#[test]
fn post_play_start_sends_expected_request() {
    let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
    let client = DawClient::new(&base_url).unwrap();

    client.post_play_start().unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("POST /play/start HTTP/1.1\r\n"));
    assert_eq!(request_body(&request), "");
}

#[test]
fn post_play_stop_sends_expected_request() {
    let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
    let client = DawClient::new(&base_url).unwrap();

    client.post_play_stop().unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("POST /play/stop HTTP/1.1\r\n"));
    assert_eq!(request_body(&request), "");
}

#[test]
fn post_daw_mode_sends_expected_request() {
    let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
    let client = DawClient::new(&base_url).unwrap();

    client.post_daw_mode().unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("POST /mode/daw HTTP/1.1\r\n"));
    assert_eq!(request_body(&request), "");
}

#[test]
fn post_ab_repeat_sends_expected_request() {
    let (base_url, request_rx) = spawn_single_request_server(r#"{"status":"ok"}"#);
    let client = DawClient::new(&base_url).unwrap();

    client.post_ab_repeat(2, 5).unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("POST /ab-repeat HTTP/1.1\r\n"));
    assert_eq!(request_body(&request), r#"{"measA":2,"measB":5}"#);
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

#[test]
fn get_status_reads_json_response() {
    let body = r#"{"mode":"daw","play":{"state":"playing","isPlaying":true,"isPreview":false,"currentMeasure":2,"currentMeasureIndex":1,"currentBeat":3,"measureElapsedMs":840,"measureDurationMs":2000,"loop":{"enabled":true,"startMeasure":1,"endMeasure":4}},"cache":{"activeRenderCount":1,"pendingCount":2,"renderingCount":1,"readyCount":5,"errorCount":0,"isUpdating":true,"isComplete":false,"cells":[[{"state":"empty"},{"state":"ready"}],[{"state":"pending"},{"state":"rendering"}]]},"grid":{"tracks":2,"measures":1}}"#;
    let (base_url, request_rx) = spawn_single_request_server(body);
    let client = DawClient::new(&base_url).unwrap();

    let status = client.get_status().unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("GET /status HTTP/1.1\r\n"));
    assert_eq!(
        status,
        DawStatusResponse {
            mode: "daw".to_string(),
            play: DawStatusPlay {
                state: "playing".to_string(),
                is_playing: true,
                is_preview: false,
                current_measure: Some(2),
                current_measure_index: Some(1),
                current_beat: Some(3),
                measure_elapsed_ms: Some(840),
                measure_duration_ms: Some(2000),
                loop_status: DawStatusLoop {
                    enabled: true,
                    start_measure: Some(1),
                    end_measure: Some(4),
                },
            },
            cache: DawStatusCache {
                active_render_count: 1,
                pending_count: 2,
                rendering_count: 1,
                ready_count: 5,
                error_count: 0,
                is_updating: true,
                is_complete: false,
                cells: vec![
                    vec![
                        DawStatusCacheCell {
                            state: "empty".to_string()
                        },
                        DawStatusCacheCell {
                            state: "ready".to_string()
                        }
                    ],
                    vec![
                        DawStatusCacheCell {
                            state: "pending".to_string()
                        },
                        DawStatusCacheCell {
                            state: "rendering".to_string()
                        }
                    ],
                ],
            },
            grid: DawStatusGrid {
                tracks: 2,
                measures: 1,
            },
        }
    );
}

#[test]
fn get_mml_reads_json_response_and_supports_init_measure_zero() {
    let (base_url, request_rx) =
        spawn_single_request_server(r#"{"track":2,"measure":0,"mml":"@1 l8cde"}"#);
    let client = DawClient::new(&base_url).unwrap();

    let mml = client.get_mml(2, 0).unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("GET /mml?track=2&measure=0 HTTP/1.1\r\n"));
    assert_eq!(mml, "@1 l8cde");
}

#[test]
fn get_mml_rejects_invalid_response_body() {
    let (base_url, _request_rx) = spawn_single_request_server(r#"{"track":2,"measure":0}"#);
    let client = DawClient::new(&base_url).unwrap();

    let error = client.get_mml(2, 0).unwrap_err();

    assert!(matches!(error, Error::InvalidResponse(_)));
}

#[test]
fn get_mmls_reads_json_response_and_etag() {
    let (base_url, request_rx) = spawn_single_request_server_with_response(
        "200 OK",
        &["Content-Type: application/json", "ETag: \"etag-1\""],
        r#"{"tracks":[["t120",""],["@1","l8cde"]]}"#,
    );
    let client = DawClient::new(&base_url).unwrap();

    let response = client.get_mmls(None).unwrap().unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("GET /mmls HTTP/1.1\r\n"));
    assert_eq!(
        response,
        GetMmlsResponse {
            etag: "\"etag-1\"".to_string(),
            tracks: vec![
                vec!["t120".to_string(), String::new()],
                vec!["@1".to_string(), "l8cde".to_string()],
            ],
        }
    );
}

#[test]
fn get_mmls_sends_if_none_match_and_returns_none_for_304() {
    let (base_url, request_rx) =
        spawn_single_request_server_with_response("304 Not Modified", &["ETag: \"etag-1\""], "");
    let client = DawClient::new(&base_url).unwrap();

    let response = client.get_mmls(Some("\"etag-1\"")).unwrap();

    let request = request_rx.recv().unwrap();
    assert!(request.starts_with("GET /mmls HTTP/1.1\r\n"));
    assert!(request
        .to_ascii_lowercase()
        .contains("\r\nif-none-match: \"etag-1\"\r\n"));
    assert_eq!(response, None);
}

#[test]
fn get_mmls_rejects_missing_etag_header() {
    let (base_url, _request_rx) = spawn_single_request_server_with_response(
        "200 OK",
        &["Content-Type: application/json"],
        r#"{"tracks":[]}"#,
    );
    let client = DawClient::new(&base_url).unwrap();

    let error = client.get_mmls(None).unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidResponse(message) if message == "missing ETag header"
    ));
}

#[test]
fn post_mml_rejects_unexpected_status_response_body() {
    let (base_url, _request_rx) = spawn_single_request_server(r#"{"status":"pending"}"#);
    let client = DawClient::new(&base_url).unwrap();

    let error = client.post_mml(2, 3, "l8cde").unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidResponse(message)
            if message == "unexpected status response (http 200): pending"
    ));
}
