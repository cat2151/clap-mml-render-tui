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
}
