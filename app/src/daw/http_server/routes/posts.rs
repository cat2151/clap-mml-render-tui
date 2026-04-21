use super::*;

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

pub(in crate::daw::http_server) fn handle_post_mml(
    mut request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_post_mixer(
    mut request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_post_patch(
    mut request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_post_patch_random(
    mut request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_post_play_start(
    request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_post_play_stop(
    request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_post_mode_daw(request: Request) {
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

pub(in crate::daw::http_server) fn handle_post_ab_repeat(
    mut request: Request,
    state: &Arc<Mutex<DawHttpState>>,
) {
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

pub(in crate::daw::http_server) fn handle_options(request: Request) {
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
