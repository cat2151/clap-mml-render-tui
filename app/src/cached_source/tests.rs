use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    sync::mpsc,
};

use super::*;

fn unique_temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cmrt-cached-source-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn accept_json(bytes: &[u8]) -> Result<()> {
    serde_json::from_slice::<serde_json::Value>(bytes)?;
    Ok(())
}

fn source(config_dir: &Path, source: &str) -> CachedSource {
    CachedSource::new("test", config_dir, source.to_string(), "cache/data.json")
}

#[test]
fn metadata_path_sits_next_to_the_data_file() {
    let spec = source(Path::new("/base"), "https://example.test/x.json");
    assert_eq!(spec.data_path, Path::new("/base").join("cache/data.json"));
    assert_eq!(
        spec.metadata_path,
        Path::new("/base").join("cache/data-http-metadata.json")
    );
}

#[test]
fn empty_source_is_disabled_and_never_reports_missing_cache() {
    let spec = source(Path::new("/base"), "   ");
    assert!(!spec.enabled());
    assert!(!spec.missing_cache());
}

#[test]
fn local_source_is_resolved_from_config_dir_and_only_rewritten_when_changed() {
    let temp = unique_temp_dir("local");
    fs::create_dir_all(temp.join("data")).unwrap();
    let source_path = temp.join("data/source.json");
    fs::write(&source_path, br#"{"a":1}"#).unwrap();
    let spec = source(&temp, "data/source.json");
    let io_lock = Mutex::new(());

    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Updated
    );
    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Unchanged,
        "内容が同じなら Unchanged"
    );

    fs::write(&source_path, br#"{"a":2}"#).unwrap();
    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Updated
    );
    assert_eq!(spec.read_cached().unwrap(), r#"{"a":2}"#);
    fs::remove_dir_all(temp).ok();
}

#[test]
fn invalid_source_keeps_the_previous_cache() {
    let temp = unique_temp_dir("invalid");
    fs::create_dir_all(&temp).unwrap();
    let source_path = temp.join("source.json");
    fs::write(&source_path, br#"{"a":1}"#).unwrap();
    let spec = source(&temp, "source.json");
    let io_lock = Mutex::new(());
    spec.refresh(&io_lock, accept_json).unwrap();

    fs::write(&source_path, b"not json").unwrap();
    assert!(spec.refresh(&io_lock, accept_json).is_err());
    assert_eq!(spec.read_cached().unwrap(), r#"{"a":1}"#);
    fs::remove_dir_all(temp).ok();
}

#[test]
fn missing_local_source_is_an_error() {
    let temp = unique_temp_dir("missing");
    let spec = source(&temp, "missing.json");
    assert!(spec.missing_cache());
    assert!(spec.refresh(&Mutex::new(()), accept_json).is_err());
    fs::remove_dir_all(temp).ok();
}

#[test]
fn url_source_uses_etag_and_preserves_json_on_304() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (request_tx, request_rx) = mpsc::channel();
    let body = r#"{"remote":true}"#.to_string();
    let served = body.clone();
    let server = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0_u8; 4096];
            let count = stream.read(&mut request).unwrap();
            request_tx
                .send(String::from_utf8_lossy(&request[..count]).to_string())
                .unwrap();
            if index == 0 {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n{}",
                    served.len(),
                    served
                )
                .unwrap();
            } else {
                stream
                    .write_all(
                        b"HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
            }
        }
    });
    let temp = unique_temp_dir("etag");
    let spec = source(&temp, &format!("http://{address}/data.json"));
    let io_lock = Mutex::new(());

    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Updated
    );
    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Unchanged
    );
    assert_eq!(spec.read_cached().unwrap(), body);
    server.join().unwrap();

    let first = request_rx.recv().unwrap();
    let second = request_rx.recv().unwrap();
    assert!(!first.to_ascii_lowercase().contains("if-none-match"));
    assert!(second.contains("If-None-Match: \"v1\""));
    fs::remove_dir_all(temp).ok();
}

#[test]
fn unchanged_remote_body_is_reported_as_unchanged() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = r#"{"same":1}"#.to_string();
    let server = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0_u8; 4096];
            // リクエストは読み捨てるが、読まずに応答するとクライアント側が壊れる。
            let _request_len = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    let temp = unique_temp_dir("same-body");
    let spec = source(&temp, &format!("http://{address}/data.json"));
    let io_lock = Mutex::new(());

    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Updated
    );
    // ETag が無いので条件付き GET は成立せず本文が返るが、内容が同じなら Unchanged。
    assert_eq!(
        spec.refresh(&io_lock, accept_json).unwrap(),
        SourceRefreshOutcome::Unchanged
    );
    server.join().unwrap();
    fs::remove_dir_all(temp).ok();
}
