use super::*;

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

    status.begin_connecting(KeyboardTransport::Http, 8);

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
    };

    status.begin_patch_setting();

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

    status.begin_patch_setting();

    assert_eq!(
        status.voicing,
        KeyboardVoicingStatus::Detecting {
            previous: Some(report.clone())
        }
    );
    assert_eq!(status.voicing.effective_decision(), report.decision);
}
