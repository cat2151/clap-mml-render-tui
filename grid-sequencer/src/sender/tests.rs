use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::TcpListener,
    time::Duration,
};

use super::*;

fn cfg_for_port(port: u16) -> cmrt_runtime::Config {
    cmrt_runtime::Config {
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
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
    }
}

/// 受け取ったリクエストのパスを記録し、すべて 204 を返すテストサーバー。
/// 起動確認の TCP 接続もそのまま受けるため、リクエスト数では止めずに動かし続ける。
fn spawn_path_recording_server() -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut first_line = String::new();
            if reader.read_line(&mut first_line).unwrap() == 0 {
                continue;
            }
            let path = first_line.split_whitespace().nth(1).unwrap().to_string();
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim_end_matches(['\r', '\n']);
                if line.is_empty() {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("Content-Length") {
                        content_length = value.trim().parse().unwrap();
                    }
                }
            }
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body).unwrap();
            write!(
                stream,
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            if tx.send(path).is_err() {
                break;
            }
        }
    });
    (port, rx)
}

/// 検証対象のエンドポイントだけを、届いた順に `wanted` 件そろうまで集める。
/// `/stop` や `/live-buffer` は接続維持のための付随リクエストなので無視する。
fn collect_endpoints(rx: &mpsc::Receiver<String>, wanted: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut paths = Vec::new();
    while paths.len() < wanted && Instant::now() < deadline {
        if let Ok(path) = rx.recv_timeout(Duration::from_millis(100)) {
            if path == "/midi" || path == "/live-patch" {
                paths.push(path);
            }
        }
    }
    paths
}

fn http_sender(port: u16) -> GridMidiSender {
    let supervisor = Arc::new(cmrt_realtime_play::RealtimePlayServerSupervisor::new(
        &cfg_for_port(port),
    ));
    GridMidiSender::new(supervisor, KeyboardTransport::Http)
}

#[test]
fn prepare_applies_the_patch_through_the_live_patch_endpoint() {
    let (port, rx) = spawn_path_recording_server();
    let sender = http_sender(port);

    sender.prepare(Some("Keys/Piano.fxp"));

    assert_eq!(collect_endpoints(&rx, 1), vec!["/live-patch".to_string()]);
}

#[test]
fn notes_are_sent_to_the_midi_endpoint() {
    let (port, rx) = spawn_path_recording_server();
    let sender = http_sender(port);

    sender.send(vec![[0x90, 60, 100]], Some("Keys/Piano.fxp"));

    assert_eq!(collect_endpoints(&rx, 1), vec!["/midi".to_string()]);
}

/// live 再生中の `/midi` は patch を無視するサーバー仕様のため、音色切替は
/// `/live-patch` を通す必要がある。加えて、切替前に旧音色で note off を
/// 送り切らないと音が鳴りっぱなしになる。順序そのものが仕様なので順序で検証する。
#[test]
fn set_patch_sends_note_offs_before_switching_the_patch() {
    let (port, rx) = spawn_path_recording_server();
    let sender = http_sender(port);

    sender.set_patch(
        vec![[0x80, 60, 0]],
        Some("Keys/Piano.fxp"),
        Some("Leads/Saw.fxp"),
    );

    assert_eq!(
        collect_endpoints(&rx, 2),
        vec!["/midi".to_string(), "/live-patch".to_string()]
    );
}

#[test]
fn set_patch_without_sounding_notes_skips_the_midi_request() {
    let (port, rx) = spawn_path_recording_server();
    let sender = http_sender(port);

    sender.set_patch(Vec::new(), None, Some("Leads/Saw.fxp"));

    assert_eq!(collect_endpoints(&rx, 1), vec!["/live-patch".to_string()]);
}
