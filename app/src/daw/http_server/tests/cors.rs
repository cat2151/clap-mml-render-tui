use super::*;

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
