use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use super::*;

#[derive(Debug)]
struct CapturedRequest {
    method: String,
    path: String,
    content_type: Option<String>,
    body: Vec<u8>,
}

fn cfg_for_port(port: u16) -> Config {
    Config {
        plugin_path: String::new(),
        input_midi: String::new(),
        output_midi: String::new(),
        output_wav: String::new(),
        sample_rate: 48_000.0,
        buffer_size: 512,
        patches_dirs: None,
        offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
        offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: crate::config::RealtimeAudioBackend::PlayServer,
        realtime_play_server_port: port,
        realtime_play_server_command: "exit 0".to_string(),
    }
}

fn spawn_one_request_server(
    status_line: &'static str,
    body: &'static str,
) -> (u16, mpsc::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let Some(request) = read_request(&mut stream) else {
                continue;
            };
            write!(
                stream,
                "{status_line}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            tx.send(request).unwrap();
            break;
        }
    });
    (port, rx)
}

fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
    let mut reader = BufReader::new(stream);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).ok()? == 0 {
        return None;
    }
    if first_line.trim().is_empty() {
        return None;
    }
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut content_length = 0usize;
    let mut content_type = None;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = value.trim().parse().unwrap();
        } else if name.eq_ignore_ascii_case("Content-Type") {
            content_type = Some(value.trim().to_string());
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    Some(CapturedRequest {
        method,
        path,
        content_type,
        body,
    })
}

#[test]
fn play_smf_posts_binary_body_to_play_endpoint() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 202 Accepted", "accepted");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor.play_smf(vec![0, 1, 2, 255]).unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, PLAY_SERVER_PLAY_PATH);
    assert_eq!(request.content_type.as_deref(), Some("audio/midi"));
    assert_eq!(request.body, vec![0, 1, 2, 255]);
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn stop_posts_to_stop_endpoint_without_spawning_when_server_is_listening() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 204 No Content", "");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor.stop().unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, PLAY_SERVER_STOP_PATH);
    assert!(request.body.is_empty());
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn stop_without_running_server_does_not_spawn_child() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor.stop().unwrap();

    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn server_error_body_is_returned() {
    let (port, _rx) = spawn_one_request_server("HTTP/1.1 415 Unsupported Media Type", "bad type");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    let error = supervisor.play_smf(vec![0]).unwrap_err();

    assert!(error
        .to_string()
        .contains("realtime play server returned HTTP 415: bad type"));
}
