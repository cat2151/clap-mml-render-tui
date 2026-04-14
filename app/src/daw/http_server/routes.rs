use std::{
    io::Read,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tiny_http::{Header, Request, Response, StatusCode};

use super::{DawHttpCommand, DawHttpCommandKind, DawHttpState};

const MAX_JSON_BODY_BYTES: u64 = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const PREFLIGHT_MAX_AGE_SECONDS: &str = "600";
const ALLOWED_CORS_ORIGINS: [&str; 2] = ["https://cat2151.github.io", "http://localhost:5173"];

#[derive(Deserialize)]
struct PostMmlRequest {
    track: usize,
    measure: usize,
    mml: String,
}

#[derive(Deserialize)]
struct PostMixerRequest {
    track: usize,
    db: f64,
}

#[derive(Deserialize)]
struct PostPatchRequest {
    track: usize,
    patch: String,
}

#[derive(Serialize)]
struct JsonStatusResponse<'a> {
    status: &'a str,
}

#[derive(Serialize)]
struct GetMmlResponse {
    track: usize,
    measure: usize,
    mml: String,
}

pub(super) fn handle_post_mml(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match read_json_body::<PostMmlRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::Mml {
                track: body.track,
                measure: body.measure,
                mml: body.mml,
            },
            cors_origin.as_deref(),
        ),
        Err((status, message)) => {
            let _ = request.respond(with_cors_headers(
                text_response(status, message),
                cors_origin.as_deref(),
            ));
        }
    }
}

pub(super) fn handle_post_mixer(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match read_json_body::<PostMixerRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::Mixer {
                track: body.track,
                db: body.db,
            },
            cors_origin.as_deref(),
        ),
        Err((status, message)) => {
            let _ = request.respond(with_cors_headers(
                text_response(status, message),
                cors_origin.as_deref(),
            ));
        }
    }
}

pub(super) fn handle_post_patch(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match read_json_body::<PostPatchRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::Patch {
                track: body.track,
                patch: body.patch,
            },
            cors_origin.as_deref(),
        ),
        Err((status, message)) => {
            let _ = request.respond(with_cors_headers(
                text_response(status, message),
                cors_origin.as_deref(),
            ));
        }
    }
}

pub(super) fn handle_get_mml(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match parse_get_mml_query(request.url()) {
        Ok((track, measure)) => {
            let mml = {
                let state = state.lock().unwrap();
                state
                    .grid_snapshot
                    .get(track)
                    .and_then(|row| row.get(measure))
                    .cloned()
                    .unwrap_or_default()
            };
            let _ = request.respond(with_cors_headers(
                json_response(
                    200,
                    &GetMmlResponse {
                        track,
                        measure,
                        mml,
                    },
                ),
                cors_origin.as_deref(),
            ));
        }
        Err((status, message)) => {
            let _ = request.respond(with_cors_headers(
                text_response(status, message),
                cors_origin.as_deref(),
            ));
        }
    }
}

pub(super) fn handle_get_patches(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    let cfg = {
        let state = state.lock().unwrap();
        state.cfg.clone()
    };
    let Some(cfg) = cfg else {
        let _ = request.respond(with_cors_headers(
            text_response(503, "DAW モードがアクティブではありません\n".to_string()),
            cors_origin.as_deref(),
        ));
        return;
    };

    if !crate::patches::has_configured_patch_dirs(cfg.as_ref()) {
        let empty = Vec::<String>::new();
        let _ = request.respond(with_cors_headers(
            json_response(200, &empty),
            cors_origin.as_deref(),
        ));
        return;
    }

    match crate::patches::collect_patch_pairs(cfg.as_ref()) {
        Ok(pairs) => {
            let patches = pairs
                .into_iter()
                .map(|(patch_name, _)| patch_name)
                .collect::<Vec<_>>();
            let _ = request.respond(with_cors_headers(
                json_response(200, &patches),
                cors_origin.as_deref(),
            ));
        }
        Err(error) => {
            let _ = request.respond(with_cors_headers(
                text_response(500, format!("patch 一覧の取得に失敗しました: {error}\n")),
                cors_origin.as_deref(),
            ));
        }
    }
}

pub(super) fn parse_get_mml_query(url: &str) -> Result<(usize, usize), (u16, String)> {
    let Some((_, query)) = url.split_once('?') else {
        return Err((400, "track と measure を指定してください\n".to_string()));
    };
    let mut track = None;
    let mut measure = None;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "track" => track = Some(parse_query_usize("track", value)?),
            "measure" | "meas" => measure = Some(parse_query_usize("measure", value)?),
            _ => {}
        }
    }
    match (track, measure) {
        (Some(track), Some(measure)) => Ok((track, measure)),
        _ => Err((400, "track と measure を指定してください\n".to_string())),
    }
}

fn parse_query_usize(name: &str, value: &str) -> Result<usize, (u16, String)> {
    if value.is_empty() {
        return Err((400, format!("{name} を指定してください\n")));
    }
    value
        .parse::<usize>()
        .map_err(|_| (400, format!("{name} は 0 以上の整数を指定してください\n")))
}

pub(super) fn handle_options(request: Request) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    let response =
        with_preflight_cors_headers(text_response(204, String::new()), cors_origin.as_deref());
    let _ = request.respond(response);
}

pub(super) fn request_origin(headers: &[Header]) -> Option<String> {
    headers
        .iter()
        .find(|header| header.field.equiv("Origin"))
        .map(|header| header.value.as_str().to_string())
}

pub(super) fn is_allowed_cors_origin(origin: &str) -> bool {
    ALLOWED_CORS_ORIGINS.contains(&origin)
}

fn validate_cors_request(
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

pub(super) fn with_cors_headers(
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
        .with_header(Header::from_bytes("Vary", "Origin").expect("valid vary header"))
}

pub(super) fn with_preflight_cors_headers(
    response: Response<std::io::Cursor<Vec<u8>>>,
    cors_origin: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    with_cors_headers(response, cors_origin)
        .with_header(
            Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .expect("valid access-control-allow-methods header"),
        )
        .with_header(
            Header::from_bytes("Access-Control-Allow-Headers", "Content-Type")
                .expect("valid access-control-allow-headers header"),
        )
        .with_header(
            Header::from_bytes("Access-Control-Max-Age", PREFLIGHT_MAX_AGE_SECONDS)
                .expect("valid access-control-max-age header"),
        )
}

fn respond_command(
    request: Request,
    state: &Arc<Mutex<DawHttpState>>,
    kind: DawHttpCommandKind,
    cors_origin: Option<&str>,
) {
    let (response_tx, response_rx) = mpsc::channel();
    {
        let mut state = state.lock().unwrap();
        state
            .pending_commands
            .push_back(DawHttpCommand { kind, response_tx });
    }

    match response_rx.recv_timeout(COMMAND_TIMEOUT) {
        Ok(Ok(())) => {
            let _ = request.respond(with_cors_headers(
                json_response(200, &JsonStatusResponse { status: "ok" }),
                cors_origin,
            ));
        }
        Ok(Err(message)) => {
            let _ = request.respond(with_cors_headers(
                text_response(400, format!("{message}\n")),
                cors_origin,
            ));
        }
        Err(_) => {
            let _ = request.respond(with_cors_headers(
                text_response(504, "DAW 反映待ちがタイムアウトしました\n".to_string()),
                cors_origin,
            ));
        }
    }
}

fn read_json_body<T: for<'de> Deserialize<'de>>(request: &mut Request) -> Result<T, (u16, String)> {
    let mut body = String::new();
    let reader = request.as_reader().take(MAX_JSON_BODY_BYTES + 1);
    let read_result = std::io::BufReader::new(reader).read_to_string(&mut body);
    if body.len() as u64 > MAX_JSON_BODY_BYTES {
        return Err((413, "リクエスト body が大きすぎます".to_string()));
    }
    if let Err(error) = read_result {
        return Err((400, format!("body の読み取りに失敗しました: {error}")));
    }
    serde_json::from_str(&body)
        .map_err(|error| (400, format!("JSON のパースに失敗しました: {error}")))
}

pub(super) fn text_response(status: u16, body: String) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes("Content-Type", "text/plain; charset=utf-8")
        .expect("valid text response header");
    Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(header)
}

fn json_response<T: Serialize>(status: u16, body: &T) -> Response<std::io::Cursor<Vec<u8>>> {
    let header =
        Header::from_bytes("Content-Type", "application/json").expect("valid json response header");
    Response::from_string(serde_json::to_string(body).expect("json response serialization"))
        .with_status_code(StatusCode(status))
        .with_header(header)
}
