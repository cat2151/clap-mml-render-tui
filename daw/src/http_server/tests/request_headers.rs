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
