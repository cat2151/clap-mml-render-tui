use std::sync::{Arc, Mutex};

use tiny_http::Request;

use super::{
    get_snapshot_mml, get_snapshot_mmls, get_status_snapshot, if_none_match_matches, json_response,
    parse_get_mml_query, request_header_value, snapshot_mmls_etag, text_response,
    validate_cors_request, with_cors_headers, with_etag_header, with_preflight_cors_headers,
    DawHttpCommandKind, DawHttpState, GetMmlResponse, GetMmlsResponse, GetStatusResponse,
    JsonStatusResponse, PostAbRepeatRequest, PostMixerRequest, PostMmlRequest, PostPatchRequest,
    PostRandomPatchRequest, RequestHeaderName,
};

pub(crate) fn handle_post_mml(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match super::read_json_body::<PostMmlRequest>(&mut request) {
        Ok(body) => super::respond_command(
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

pub(crate) fn handle_post_mixer(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match super::read_json_body::<PostMixerRequest>(&mut request) {
        Ok(body) => super::respond_command(
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

pub(crate) fn handle_post_patch(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match super::read_json_body::<PostPatchRequest>(&mut request) {
        Ok(body) => super::respond_command(
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

pub(crate) fn handle_post_patch_random(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match super::read_json_body::<PostRandomPatchRequest>(&mut request) {
        Ok(body) => super::respond_command(
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

pub(crate) fn handle_post_play_start(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    super::respond_command(
        request,
        state,
        DawHttpCommandKind::PlayStart,
        cors_origin.as_deref(),
    );
}

pub(crate) fn handle_post_play_stop(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    super::respond_command(
        request,
        state,
        DawHttpCommandKind::PlayStop,
        cors_origin.as_deref(),
    );
}

pub(crate) fn handle_post_mode_daw(request: Request) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    super::super::request_daw_mode_switch();
    let _ = request.respond(with_cors_headers(
        json_response(200, &JsonStatusResponse { status: "ok" }),
        cors_origin.as_deref(),
    ));
}

pub(crate) fn handle_post_ab_repeat(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    match super::read_json_body::<PostAbRepeatRequest>(&mut request) {
        Ok(body) => super::respond_command(
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

pub(crate) fn handle_get_mml(request: Request, state: &Arc<Mutex<DawHttpState>>) {
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

pub(crate) fn handle_get_mmls(request: Request, state: &Arc<Mutex<DawHttpState>>) {
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
                    with_etag_header(super::empty_response(304), &etag),
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

pub(crate) fn handle_get_patches(request: Request, state: &Arc<Mutex<DawHttpState>>) {
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

pub(crate) fn handle_get_status(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cors_origin = match validate_cors_request(&request) {
        Ok(cors_origin) => cors_origin,
        Err(response) => {
            let _ = request.respond(response);
            return;
        }
    };
    let status_result = {
        let state = state.lock().unwrap();
        get_status_snapshot(&state)
    };
    let response = match status_result {
        Ok(snapshot) => with_cors_headers(
            json_response(200, &GetStatusResponse::from(snapshot)),
            cors_origin.as_deref(),
        ),
        Err((status, message)) => {
            with_cors_headers(text_response(status, message), cors_origin.as_deref())
        }
    };
    let _ = request.respond(response);
}

pub(crate) fn handle_options(request: Request) {
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
