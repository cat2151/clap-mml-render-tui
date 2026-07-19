use std::time::Duration;

use super::*;
use crate::realtime_play::VoicingReport;

use std::{
    io::{BufRead as _, BufReader, Read as _, Write as _},
    net::TcpListener,
};

fn cfg_for_port(port: u16) -> crate::config::Config {
    crate::config::Config {
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

/// cache に判定結果がある patch では probe エンドポイントを叩かないことを確認する。
/// probe を省けるかどうかがこの機能の目的そのものなので、UI 表示ではなく
/// 実際に送るリクエスト先で検証する。
#[test]
fn prepare_connection_with_cached_voicing_skips_the_probe_endpoint() {
    let (port, rx) = spawn_path_recording_server();
    let supervisor = RealtimePlayServerSupervisor::new(&cfg_for_port(port));
    let mut worker = WorkerState::new(KeyboardTransport::Http, 4);
    let status = Mutex::new(KeyboardConnectionStatus::default());

    let result = prepare_connection(
        &mut worker,
        &supervisor,
        &status,
        PatchRequest {
            patch: Some("Leads/Mono.fxp"),
            known_voicing: Some(PatchVoicing::Mono),
        },
        1,
    )
    .unwrap();

    assert!(result.is_none(), "probe を行っていないのでレポートはない");
    let paths = rx.try_iter().collect::<Vec<_>>();
    assert!(
        paths.contains(&"/live-patch".to_string()),
        "patch 適用は行う: {paths:?}"
    );
    assert!(
        !paths.contains(&"/live-patch-probe".to_string()),
        "probe は行わない: {paths:?}"
    );
}

#[test]
fn transport_toggle_is_symmetric() {
    assert_eq!(
        KeyboardTransport::Http.toggled(),
        KeyboardTransport::SharedMemory
    );
    assert_eq!(
        KeyboardTransport::SharedMemory.toggled(),
        KeyboardTransport::Http
    );
}

#[test]
fn default_status_starts_with_shm_x4() {
    let status = KeyboardConnectionStatus::default();
    assert_eq!(status.transport, KeyboardTransport::SharedMemory);
    assert_eq!(status.phase, KeyboardConnectionPhase::Idle);
    assert_eq!(status.last_send, None);
    assert_eq!(status.buffer_multiplier, 4);
    assert_eq!(status.voicing, KeyboardVoicingStatus::Unavailable);
}

#[test]
fn begin_connecting_updates_initialization_status_synchronously() {
    let mut status = KeyboardConnectionStatus {
        last_send: Some(Duration::from_millis(12)),
        ..KeyboardConnectionStatus::default()
    };

    status.begin_connecting(KeyboardTransport::Http, 8, Some("Leads/Mono.fxp"), None);

    assert_eq!(status.transport, KeyboardTransport::Http);
    assert_eq!(status.phase, KeyboardConnectionPhase::Connecting);
    assert_eq!(status.last_send, None);
    assert_eq!(status.buffer_multiplier, 8);
    assert_eq!(
        status.voicing,
        KeyboardVoicingStatus::Detecting { previous: None }
    );
}

#[test]
fn begin_patch_setting_preserves_transport_and_buffer() {
    let mut status = KeyboardConnectionStatus {
        transport: KeyboardTransport::Http,
        buffer_multiplier: 8,
        phase: KeyboardConnectionPhase::Ready,
        last_send: Some(Duration::from_millis(12)),
        voicing: KeyboardVoicingStatus::Unavailable,
        voicing_patch: None,
    };

    status.begin_patch_setting(Some("Leads/Mono.fxp"), None);

    assert_eq!(status.transport, KeyboardTransport::Http);
    assert_eq!(status.buffer_multiplier, 8);
    assert_eq!(status.phase, KeyboardConnectionPhase::PatchSetting);
    assert_eq!(status.last_send, Some(Duration::from_millis(12)));
    assert_eq!(
        status.voicing,
        KeyboardVoicingStatus::Detecting { previous: None }
    );
}

#[test]
fn begin_patch_setting_keeps_the_previous_detection_visible() {
    let report: VoicingReport = serde_json::from_value(serde_json::json!({
        "decision": "mono",
        "probe": {"result": "mono", "ended_note_ids": [2], "blocks": 1},
        "voice_info": null,
        "surge": null,
        "disagreement": false
    }))
    .unwrap();
    let mut status = KeyboardConnectionStatus {
        voicing: KeyboardVoicingStatus::Detected(report.clone()),
        ..KeyboardConnectionStatus::default()
    };

    status.begin_patch_setting(Some("Leads/Mono.fxp"), None);

    assert_eq!(
        status.voicing,
        KeyboardVoicingStatus::Detecting {
            previous: Some(report.clone())
        }
    );
    assert_eq!(status.voicing.effective_decision(), report.decision);
}

#[test]
fn begin_patch_setting_with_cached_voicing_skips_probing() {
    let mut status = KeyboardConnectionStatus::default();

    status.begin_patch_setting(Some("Leads/Mono.fxp"), Some(PatchVoicing::Mono));

    assert_eq!(
        status.voicing,
        KeyboardVoicingStatus::Cached(PatchVoicing::Mono)
    );
    assert_eq!(status.voicing.effective_decision(), PatchVoicing::Mono);
    assert_eq!(status.voicing_patch.as_deref(), Some("Leads/Mono.fxp"));
}
