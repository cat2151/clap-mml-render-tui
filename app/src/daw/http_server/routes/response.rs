use tiny_http::{Header, Request, Response, StatusCode};

const PREFLIGHT_MAX_AGE_SECONDS: &str = "600";
const ALLOWED_CORS_ORIGINS: [&str; 2] = ["https://cat2151.github.io", "http://localhost:5173"];

#[derive(Clone, Copy)]
pub(in crate::daw::http_server) enum RequestHeaderName {
    Origin,
    IfNoneMatch,
}

pub(in crate::daw::http_server) fn request_header_value(
    headers: &[Header],
    name: RequestHeaderName,
) -> Option<String> {
    headers
        .iter()
        .find(|header| match name {
            RequestHeaderName::Origin => header.field.equiv("Origin"),
            RequestHeaderName::IfNoneMatch => header.field.equiv("If-None-Match"),
        })
        .map(|header| header.value.as_str().to_string())
}

pub(in crate::daw::http_server) fn request_origin(headers: &[Header]) -> Option<String> {
    request_header_value(headers, RequestHeaderName::Origin)
}

pub(in crate::daw::http_server) fn is_allowed_cors_origin(origin: &str) -> bool {
    ALLOWED_CORS_ORIGINS.contains(&origin)
}

pub(in crate::daw::http_server) fn validate_cors_request(
    request: &Request,
) -> Result<Option<String>, Response<std::io::Cursor<Vec<u8>>>> {
    let Some(origin) = request_origin(request.headers()) else {
        return Ok(None);
    };
    if is_allowed_cors_origin(&origin) {
        return Ok(Some(origin));
    }
    Err(text_response(
        403,
        format!("Origin が許可されていません: {origin}\n"),
    ))
}

pub(in crate::daw::http_server) fn with_cors_headers(
    response: Response<std::io::Cursor<Vec<u8>>>,
    cors_origin: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(origin) = cors_origin else {
        return response;
    };
    response
        .with_header(
            Header::from_bytes("Access-Control-Allow-Origin", origin)
                .expect("valid access-control-allow-origin header"),
        )
        .with_header(
            Header::from_bytes("Access-Control-Expose-Headers", "ETag")
                .expect("valid access-control-expose-headers header"),
        )
        .with_header(Header::from_bytes("Vary", "Origin").expect("valid vary header"))
}

pub(in crate::daw::http_server) fn with_preflight_cors_headers(
    response: Response<std::io::Cursor<Vec<u8>>>,
    cors_origin: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    with_cors_headers(response, cors_origin)
        .with_header(
            Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .expect("valid access-control-allow-methods header"),
        )
        .with_header(
            Header::from_bytes(
                "Access-Control-Allow-Headers",
                "Content-Type, If-None-Match",
            )
            .expect("valid access-control-allow-headers header"),
        )
        .with_header(
            Header::from_bytes("Access-Control-Max-Age", PREFLIGHT_MAX_AGE_SECONDS)
                .expect("valid access-control-max-age header"),
        )
}

pub(in crate::daw::http_server) fn with_etag_header(
    response: Response<std::io::Cursor<Vec<u8>>>,
    etag: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    response.with_header(Header::from_bytes("ETag", etag).expect("valid etag header"))
}

pub(in crate::daw::http_server) fn text_response(
    status: u16,
    body: String,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes("Content-Type", "text/plain; charset=utf-8")
        .expect("valid text response header");
    Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(header)
}

pub(in crate::daw::http_server) fn empty_response(
    status: u16,
) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(Vec::new()).with_status_code(StatusCode(status))
}

pub(in crate::daw::http_server) fn json_response<T: serde::Serialize>(
    status: u16,
    body: &T,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let header =
        Header::from_bytes("Content-Type", "application/json").expect("valid json response header");
    Response::from_string(serde_json::to_string(body).expect("json response serialization"))
        .with_status_code(StatusCode(status))
        .with_header(header)
}
