use super::*;

#[test]
fn claim_http_server_thread_slot_is_reusable_after_drop() {
    let _test_guard = lock_http_server_test_state();
    let first_guard = claim_http_server_thread_slot().expect("first claim should succeed");
    assert!(
        claim_http_server_thread_slot().is_none(),
        "second concurrent claim should fail"
    );
    drop(first_guard);
    assert!(
        claim_http_server_thread_slot().is_some(),
        "slot should be reusable after guard drop"
    );
}

#[test]
fn daw_mode_switch_request_is_consumed_once() {
    let _test_guard = lock_http_server_test_state();
    deactivate_daw_http_server();
    assert!(!take_daw_mode_switch_request());

    request_daw_mode_switch();

    assert!(take_daw_mode_switch_request());
    assert!(!take_daw_mode_switch_request());
}

#[test]
fn daw_mode_switch_request_is_ignored_while_daw_is_active() {
    let _test_guard = lock_http_server_test_state();
    deactivate_daw_http_server();
    assert!(!take_daw_mode_switch_request());
    activate_http_state(build_http_state(default_config()));

    request_daw_mode_switch();

    assert!(!take_daw_mode_switch_request());
    deactivate_daw_http_server();
    assert!(!take_daw_mode_switch_request());
}
