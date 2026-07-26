use super::*;

#[test]
fn default_status_is_shm_ready_for_instance_zero_workflow() {
    let status = KeyboardConnectionStatus::default();
    assert_eq!(status.phase, KeyboardConnectionPhase::Idle);
    assert_eq!(status.buffer_multiplier, 4);
}

#[test]
fn begin_connecting_tracks_patch_and_cached_voicing() {
    let mut status = KeyboardConnectionStatus::default();
    status.begin_connecting(8, Some("Leads/Mono.fxp"), Some(PatchVoicing::Mono));

    assert_eq!(status.phase, KeyboardConnectionPhase::Connecting);
    assert_eq!(status.buffer_multiplier, 8);
    assert_eq!(status.voicing_patch.as_deref(), Some("Leads/Mono.fxp"));
    assert_eq!(
        status.voicing,
        KeyboardVoicingStatus::Cached(PatchVoicing::Mono)
    );
}
