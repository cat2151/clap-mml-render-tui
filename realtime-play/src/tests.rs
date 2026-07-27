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
        loop_dirs: Vec::new(),
        loop_categories: cmrt_runtime::default_loop_categories(),
        offline_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
        offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::PlayServer,
        realtime_play_server_port: port,
        realtime_play_server_command: "exit 0".to_string(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
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
        loop {
            let (mut stream, _) = listener.accept().unwrap();
            // Supervisorの起動確認はTCP接続だけ行うため、その接続は読み飛ばす。
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
    reader.read_line(&mut first_line).ok()?;
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
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = value.trim().parse().unwrap();
            } else if name.eq_ignore_ascii_case("Content-Type") {
                content_type = Some(value.trim().to_string());
            }
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
}

#[test]
fn play_mml_posts_text_body_to_play_mml_endpoint() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 202 Accepted", "accepted");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));
    let mml = "{\"Surge XT patch\": \"Keys/DX EP.fxp\"}cde";
    supervisor.play_mml(mml, vec![0, 1, 2]).unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.path, PLAY_SERVER_PLAY_MML_PATH);
    assert_eq!(request.content_type.as_deref(), Some(PLAY_CONTENT_TYPE_MML));
    assert_eq!(request.body, mml.as_bytes());
}

#[test]
fn http_stop_remains_for_scheduled_playback() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 204 No Content", "");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));
    supervisor.stop().unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.path, PLAY_SERVER_STOP_PATH);
    assert!(request.body.is_empty());
}

#[test]
fn configured_server_command_description_is_explicit() {
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(62_154));
    let (_, description) = supervisor.build_command();

    assert_eq!(description, "source=config shell_command=\"exit 0\"");
}
