use super::*;

#[test]
fn default_status_starts_idle_on_shared_memory() {
    let status = GridConnectionStatus::default();
    assert_eq!(status.transport, KeyboardTransport::SharedMemory);
    assert_eq!(status.phase, GridConnectionPhase::Idle);
    assert_eq!(status.last_send, None);
    assert_eq!(status.patch, None);
}

#[test]
fn only_the_ready_phase_accepts_notes() {
    assert!(GridConnectionPhase::Ready.accepts_notes());
    assert!(!GridConnectionPhase::Idle.accepts_notes());
    assert!(!GridConnectionPhase::Connecting.accepts_notes());
    assert!(!GridConnectionPhase::PatchSetting.accepts_notes());
    assert!(!GridConnectionPhase::Error("boom".to_string()).accepts_notes());
}

#[test]
fn begin_connecting_clears_the_previous_send_time() {
    let mut status = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        last_send: Some(Duration::from_millis(12)),
        ..GridConnectionStatus::default()
    };

    status.begin_connecting(Some("Keys/Piano.fxp"));

    assert_eq!(status.phase, GridConnectionPhase::Connecting);
    assert_eq!(status.last_send, None);
    assert_eq!(status.patch.as_deref(), Some("Keys/Piano.fxp"));
}

#[test]
fn begin_patch_setting_keeps_the_last_send_time() {
    let mut status = GridConnectionStatus {
        phase: GridConnectionPhase::Ready,
        last_send: Some(Duration::from_millis(12)),
        ..GridConnectionStatus::default()
    };

    status.begin_patch_setting(Some("Leads/Saw.fxp"));

    assert_eq!(status.phase, GridConnectionPhase::PatchSetting);
    assert_eq!(status.last_send, Some(Duration::from_millis(12)));
    assert_eq!(status.patch.as_deref(), Some("Leads/Saw.fxp"));
}

#[test]
fn a_successful_send_becomes_ready_and_a_successful_stop_becomes_idle() {
    let mut status = GridConnectionStatus::default();

    status.apply_result(Ok(()), Some(Duration::from_millis(3)), false);
    assert_eq!(status.phase, GridConnectionPhase::Ready);
    assert_eq!(status.last_send, Some(Duration::from_millis(3)));

    status.apply_result(Ok(()), None, true);
    assert_eq!(status.phase, GridConnectionPhase::Idle);
}

#[test]
fn a_failure_surfaces_the_error_message() {
    let mut status = GridConnectionStatus::default();

    status.apply_result(Err(anyhow::anyhow!("connection refused")), None, false);

    assert_eq!(
        status.phase,
        GridConnectionPhase::Error("connection refused".to_string())
    );
    assert_eq!(status.phase.label(), "connection refused");
}
