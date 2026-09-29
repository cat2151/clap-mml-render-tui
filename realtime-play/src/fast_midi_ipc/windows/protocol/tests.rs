use super::*;

#[test]
fn patch_loading_requests_wait_longer_than_acceptance_only_requests() {
    assert_eq!(
        response_timeout(KIND_PREPARE_PATCH),
        Duration::from_secs(300)
    );
    assert_eq!(response_timeout(KIND_PROBE_PATCH), Duration::from_secs(300));
    assert_eq!(
        response_timeout(KIND_PREPARE_STANDBY_PATCH),
        Duration::from_secs(30)
    );
}
