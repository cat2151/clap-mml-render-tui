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
        chord_progression_source: String::new(),
        ..Default::default()
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

#[test]
fn live_instance_counts_normalize_and_cycle() {
    assert_eq!(normalize_live_instance_count(0), 16);
    assert_eq!(normalize_live_instance_count(5), 16);
    assert_eq!(normalize_live_instance_count(8), 8);
    // 3 は chord mode（和音 / bass / アルペジオ）用に足した値。
    assert_eq!(normalize_live_instance_count(3), 3);
    // 7 は drum（chord / bass / アルペジオ + drum 4 role）用に足した値。
    assert_eq!(normalize_live_instance_count(7), 7);
    assert_eq!(
        [1, 2, 3, 4, 7, 8, 16].map(next_live_instance_count),
        [2, 3, 4, 7, 8, 16, 1]
    );
}

/// 各トラックは bank 2 本ぶんの instance を使う。UI が見せるトラック数と、
/// サーバーが生成する instance 数を取り違えないための番人。
#[test]
fn each_track_takes_two_instances_for_double_buffering() {
    assert_eq!(server_instance_count(1), 2);
    assert_eq!(server_instance_count(3), 6);
    assert_eq!(server_instance_count(16), 32);
    // 未対応の値はトラック数の既定へ丸めてから 2 倍する。
    assert_eq!(server_instance_count(5), 32);
    for tracks in SUPPORTED_LIVE_INSTANCE_COUNTS {
        assert!(SUPPORTED_SERVER_INSTANCE_COUNTS.contains(&server_instance_count(tracks)));
    }
}

#[test]
fn supervisor_keeps_requested_live_instance_count() {
    let supervisor =
        RealtimePlayServerSupervisor::with_live_instance_count(&cfg_for_port(62_154), 4);
    assert_eq!(supervisor.live_instance_count(), 4);
    let (command, _) = supervisor.build_command();
    assert_eq!(
        command
            .get_envs()
            .find(|(name, _)| *name == LIVE_INSTANCE_COUNT_ENV)
            .and_then(|(_, value)| value)
            .and_then(std::ffi::OsStr::to_str),
        Some("4")
    );
}
