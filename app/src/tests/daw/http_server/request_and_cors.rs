use super::*;

#[test]
fn request_origin_extracts_origin_header() {
    let header = tiny_http::Header::from_bytes("Origin", "https://cat2151.github.io").unwrap();

    assert_eq!(
        request_origin(&[header]),
        Some("https://cat2151.github.io".to_string())
    );
    assert_eq!(request_origin(&[]), None);
}

#[test]
fn request_header_value_extracts_case_insensitive_header() {
    let header = tiny_http::Header::from_bytes("If-None-Match", "\"abc123\"").unwrap();

    assert_eq!(
        request_header_value(&[header], RequestHeaderName::IfNoneMatch),
        Some("\"abc123\"".to_string())
    );
}

#[test]
fn is_allowed_cors_origin_accepts_known_origins() {
    assert!(is_allowed_cors_origin("https://cat2151.github.io"));
    assert!(is_allowed_cors_origin("http://localhost:5173"));
    assert!(!is_allowed_cors_origin("https://example.com"));
}

#[test]
fn with_cors_headers_adds_origin_and_vary_headers() {
    let response = with_cors_headers(
        tiny_http::Response::from_string("ok"),
        Some("https://cat2151.github.io"),
    );

    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Allow-Origin")
            && header.value.as_str() == "https://cat2151.github.io"));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Expose-Headers")
            && header.value.as_str() == "ETag"));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Vary") && header.value.as_str() == "Origin"));
}

#[test]
fn with_preflight_cors_headers_adds_preflight_headers() {
    let response = with_preflight_cors_headers(
        tiny_http::Response::from_string(""),
        Some("http://localhost:5173"),
    );

    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Allow-Methods")));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Allow-Headers")
            && header.value.as_str().contains("If-None-Match")));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Max-Age")));
}

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
