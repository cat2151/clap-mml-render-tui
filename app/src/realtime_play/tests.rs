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

/// 応答を順番に返すテストサーバー。応答を返し切ったら終了する。
/// keep-alive による接続再利用を避けるため Connection: close を返す。
fn spawn_sequential_response_server(
    responses: Vec<(&'static str, &'static str)>,
) -> (u16, mpsc::Receiver<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut responses = responses.into_iter();
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let Some(request) = read_request(&mut stream) else {
                continue;
            };
            let Some((status_line, body)) = responses.next() else {
                break;
            };
            write!(
                stream,
                "{status_line}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            tx.send(request).unwrap();
            if responses.len() == 0 {
                break;
            }
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
fn play_mml_posts_text_body_to_play_mml_endpoint() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 202 Accepted", "accepted");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    let mml = "{\"Surge XT patch\": \"Keys/DX EP.fxp\"}cde";
    supervisor.play_mml(mml, vec![0, 1, 2]).unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, PLAY_SERVER_PLAY_MML_PATH);
    assert_eq!(request.content_type.as_deref(), Some(PLAY_CONTENT_TYPE_MML));
    assert_eq!(request.body, mml.as_bytes());
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn send_midi_posts_ordered_json_batch() {
    let (port, rx) = spawn_sequential_response_server(vec![
        ("HTTP/1.1 202 Accepted", "accepted"),
        ("HTTP/1.1 202 Accepted", "accepted"),
    ]);
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor
        .send_midi(&[[0x80, 60, 0], [0x90, 62, 100]], None)
        .unwrap();

    let buffer_request = rx.recv().unwrap();
    assert_eq!(buffer_request.path, "/live-buffer");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&buffer_request.body).unwrap(),
        serde_json::json!({"multiplier": 4})
    );
    let request = rx.recv().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, PLAY_SERVER_MIDI_PATH);
    assert_eq!(
        request.content_type.as_deref(),
        Some(PLAY_CONTENT_TYPE_JSON)
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"messages": [[128, 60, 0], [144, 62, 100]]})
    );
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn send_midi_includes_selected_patch() {
    let (port, rx) = spawn_sequential_response_server(vec![
        ("HTTP/1.1 202 Accepted", "accepted"),
        ("HTTP/1.1 202 Accepted", "accepted"),
    ]);
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));
    supervisor.remember_live_buffer_multiplier(4).unwrap();

    supervisor
        .send_midi(&[[0x90, 60, 100]], Some("patches_factory/Keys/Piano.fxp"))
        .unwrap();

    let buffer_request = rx.recv().unwrap();
    assert_eq!(buffer_request.path, "/live-buffer");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&buffer_request.body).unwrap(),
        serde_json::json!({"multiplier": 4})
    );
    let request = rx.recv().unwrap();
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "messages": [[144, 60, 100]],
            "patch": "patches_factory/Keys/Piano.fxp"
        })
    );
}

#[test]
fn set_live_buffer_multiplier_posts_selected_depth() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 202 Accepted", "accepted");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor.set_live_buffer_multiplier(8).unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.path, "/live-buffer");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        serde_json::json!({"multiplier": 8})
    );
}

#[test]
fn prepare_live_patch_posts_selected_patch_and_waits_for_response() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 204 No Content", "");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor
        .prepare_live_patch(Some("patches_factory/Keys/Piano.fxp"))
        .unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/live-patch");
    assert_eq!(
        request.content_type.as_deref(),
        Some(PLAY_CONTENT_TYPE_JSON)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        serde_json::json!({"patch": "patches_factory/Keys/Piano.fxp"})
    );
}

#[test]
fn prepare_live_patch_posts_null_for_init_saw() {
    let (port, rx) = spawn_one_request_server("HTTP/1.1 204 No Content", "");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor.prepare_live_patch(None).unwrap();

    let request = rx.recv().unwrap();
    assert_eq!(request.path, "/live-patch");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        serde_json::json!({"patch": null})
    );
}

#[test]
fn prepare_live_patch_with_voicing_reads_probe_report() {
    let report_json = r#"{"decision":"mono","probe":{"result":"mono","ended_note_ids":[1],"blocks":1},"voice_info":{"voice_count":128,"voice_capacity":128,"supports_overlapping_notes":true},"surge":null,"disagreement":false}"#;
    let (port, rx) = spawn_one_request_server("HTTP/1.1 200 OK", report_json);
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    let report = supervisor
        .prepare_live_patch_with_voicing(Some("Leads/Mono.fxp"))
        .unwrap()
        .unwrap();

    assert_eq!(report.decision, PatchVoicing::Mono);
    assert_eq!(report.probe.ended_note_ids, vec![1]);
    let request = rx.recv().unwrap();
    assert_eq!(request.path, "/live-patch-probe");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
        serde_json::json!({"patch": "Leads/Mono.fxp"})
    );
}

#[test]
fn voicing_probe_falls_back_to_legacy_patch_endpoint() {
    let (port, rx) = spawn_sequential_response_server(vec![
        ("HTTP/1.1 404 Not Found", "not found"),
        ("HTTP/1.1 204 No Content", ""),
    ]);
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    let report = supervisor
        .prepare_live_patch_with_voicing(Some("Leads/Mono.fxp"))
        .unwrap();

    assert_eq!(report, None);
    assert_eq!(rx.recv().unwrap().path, "/live-patch-probe");
    assert_eq!(rx.recv().unwrap().path, "/live-patch");
}

#[test]
fn play_mml_falls_back_to_play_when_server_lacks_play_mml() {
    // 旧サーバー（/play-mml 未対応 → 404）を模す。フォールバックで /play に SMF が届くこと。
    let (port, rx) = spawn_sequential_response_server(vec![
        ("HTTP/1.1 404 Not Found", "not found"),
        ("HTTP/1.1 202 Accepted", "accepted"),
    ]);
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    supervisor.play_mml("cde", vec![9, 8, 7]).unwrap();

    let first = rx.recv().unwrap();
    assert_eq!(first.path, PLAY_SERVER_PLAY_MML_PATH);
    let second = rx.recv().unwrap();
    assert_eq!(second.path, PLAY_SERVER_PLAY_PATH);
    assert_eq!(second.content_type.as_deref(), Some(PLAY_CONTENT_TYPE_MIDI));
    assert_eq!(second.body, vec![9, 8, 7]);
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn play_mml_returns_server_error_without_fallback() {
    let (port, _rx) = spawn_one_request_server("HTTP/1.1 500 Internal Server Error", "boom");
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));

    let error = supervisor.play_mml("cde", vec![0]).unwrap_err();

    assert!(error
        .to_string()
        .contains("realtime play server returned HTTP 500: boom"));
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
