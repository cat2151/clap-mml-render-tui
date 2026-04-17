use std::{
    io::Read,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tiny_http::Request;

use super::{DawHttpCommand, DawHttpCommandKind, DawHttpState};

const MAX_JSON_BODY_BYTES: u64 = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[path = "routes/response.rs"]
mod response;
#[path = "routes/snapshot.rs"]
mod snapshot;

use response::{empty_response, json_response, with_etag_header};
#[cfg(test)]
pub(super) use response::{is_allowed_cors_origin, request_origin};
pub(super) use response::{
    request_header_value, text_response, validate_cors_request, with_cors_headers,
    with_preflight_cors_headers, RequestHeaderName,
};
pub(super) use snapshot::{
    get_snapshot_mml, get_snapshot_mmls, if_none_match_matches, parse_get_mml_query,
    snapshot_mmls_etag,
};

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

#[derive(Deserialize)]
struct PostRandomPatchRequest {
    track: usize,
}

#[derive(Deserialize)]
struct PostAbRepeatRequest {
    #[serde(rename = "measA", alias = "measureA")]
    start_measure: usize,
    #[serde(rename = "measB", alias = "measureB")]
    end_measure: usize,
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

#[derive(Serialize)]
struct GetMmlsResponse {
    tracks: Vec<Vec<String>>,
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

pub(super) fn handle_post_patch_random(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match read_json_body::<PostRandomPatchRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::RandomPatch { track: body.track },
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

pub(super) fn handle_post_play_start(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    respond_command(
        request,
        state,
        DawHttpCommandKind::PlayStart,
        cors_origin.as_deref(),
    );
}

pub(super) fn handle_post_play_stop(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    respond_command(
        request,
        state,
        DawHttpCommandKind::PlayStop,
        cors_origin.as_deref(),
    );
}

pub(super) fn handle_post_mode_daw(request: Request) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    super::request_daw_mode_switch();
    let _ = request.respond(with_cors_headers(
        json_response(200, &JsonStatusResponse { status: "ok" }),
        cors_origin.as_deref(),
    ));
}

pub(super) fn handle_post_ab_repeat(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match read_json_body::<PostAbRepeatRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::AbRepeat {
                start_measure: body.start_measure,
                end_measure: body.end_measure,
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
            let mml_result = {
                let state = state.lock().unwrap();
                get_snapshot_mml(&state, track, measure)
            };
            let response = match mml_result {
                Ok(mml) => with_cors_headers(
                    json_response(
                        200,
                        &GetMmlResponse {
                            track,
                            measure,
                            mml,
                        },
                    ),
                    cors_origin.as_deref(),
                ),
                Err((status, message)) => {
                    with_cors_headers(text_response(status, message), cors_origin.as_deref())
                }
            };
            let _ = request.respond(response);
        }
        Err((status, message)) => {
            let _ = request.respond(with_cors_headers(
                text_response(status, message),
                cors_origin.as_deref(),
            ));
        }
    }
}

pub(super) fn handle_get_mmls(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    let if_none_match = request_header_value(request.headers(), RequestHeaderName::IfNoneMatch);
    let snapshot_result = {
        let state = state.lock().unwrap();
        get_snapshot_mmls(&state)
    };
    let response = match snapshot_result {
        Ok(tracks) => {
            let etag = snapshot_mmls_etag(&tracks);
            if if_none_match
                .as_deref()
                .is_some_and(|header| if_none_match_matches(header, &etag))
            {
                with_cors_headers(
                    with_etag_header(empty_response(304), &etag),
                    cors_origin.as_deref(),
                )
            } else {
                with_cors_headers(
                    with_etag_header(json_response(200, &GetMmlsResponse { tracks }), &etag),
                    cors_origin.as_deref(),
                )
            }
        }
        Err((status, message)) => {
            with_cors_headers(text_response(status, message), cors_origin.as_deref())
        }
    };
    let _ = request.respond(response);
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

#[cfg(test)]
mod tests {
    use super::PostAbRepeatRequest;

    #[test]
    fn post_ab_repeat_request_deserializes_measure_aliases() {
        let request: PostAbRepeatRequest =
            serde_json::from_str(r#"{"measureA":3,"measureB":7}"#).unwrap();

        assert_eq!(request.start_measure, 3);
        assert_eq!(request.end_measure, 7);
    }

    #[test]
    fn post_ab_repeat_request_measure_aliases_match_canonical_names() {
        let canonical: PostAbRepeatRequest =
            serde_json::from_str(r#"{"measA":5,"measB":9}"#).unwrap();
        let alias: PostAbRepeatRequest =
            serde_json::from_str(r#"{"measureA":5,"measureB":9}"#).unwrap();

        assert_eq!(alias.start_measure, canonical.start_measure);
        assert_eq!(alias.end_measure, canonical.end_measure);
    }
}
