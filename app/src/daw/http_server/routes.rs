use std::{
    io::Read,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tiny_http::Request;

use super::{DawHttpCommand, DawHttpCommandKind, DawHttpState, DawStatusSnapshot};
use crate::daw::{CacheState, DawPlayState};

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
    get_snapshot_mml, get_snapshot_mmls, get_status_snapshot, if_none_match_matches,
    parse_get_mml_query, snapshot_mmls_etag,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetStatusResponse {
    mode: &'static str,
    play: GetStatusPlayResponse,
    cache: GetStatusCacheResponse,
    grid: GetStatusGridResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetStatusPlayResponse {
    state: &'static str,
    is_playing: bool,
    is_preview: bool,
    current_measure: Option<usize>,
    current_measure_index: Option<usize>,
    current_beat: Option<u32>,
    #[serde(rename = "loop")]
    loop_status: GetStatusLoopResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetStatusLoopResponse {
    enabled: bool,
    start_measure: Option<usize>,
    end_measure: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetStatusCacheResponse {
    active_render_count: usize,
    pending_count: usize,
    rendering_count: usize,
    ready_count: usize,
    error_count: usize,
    is_updating: bool,
    is_complete: bool,
    cells: Vec<Vec<GetStatusCacheCellResponse>>,
}

#[derive(Serialize)]
struct GetStatusCacheCellResponse {
    state: &'static str,
}

#[derive(Serialize)]
struct GetStatusGridResponse {
    tracks: usize,
    measures: usize,
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

pub(super) fn handle_get_status(request: Request, state: &Arc<Mutex<DawHttpState>>) {
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

impl From<DawStatusSnapshot> for GetStatusResponse {
    fn from(snapshot: DawStatusSnapshot) -> Self {
        let current_measure_index = current_measure_index(&snapshot);
        let current_measure = current_measure_index.map(|measure_index| measure_index + 1);
        let current_beat = current_beat(&snapshot);
        let loop_range = snapshot.ab_repeat.marker_indices().map(|(start, end)| {
            (
                start.min(end).saturating_add(1),
                start.max(end).saturating_add(1),
            )
        });
        let (loop_enabled, loop_start, loop_end) = match loop_range {
            Some((start_measure, end_measure)) => (true, Some(start_measure), Some(end_measure)),
            None => (false, None, None),
        };
        let pending_count = snapshot.cache.pending_count;
        let rendering_count = snapshot.cache.rendering_count;
        let ready_count = snapshot.cache.ready_count;
        let error_count = snapshot.cache.error_count;
        let is_updating = pending_count > 0 || rendering_count > 0;
        let cells = snapshot
            .cache
            .cells
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|state| GetStatusCacheCellResponse {
                        state: cache_state_label(&state),
                    })
                    .collect()
            })
            .collect();

        Self {
            mode: "daw",
            play: GetStatusPlayResponse {
                state: play_state_label(snapshot.play_state),
                is_playing: snapshot.play_state == DawPlayState::Playing,
                is_preview: snapshot.play_state == DawPlayState::Preview,
                current_measure,
                current_measure_index,
                current_beat,
                loop_status: GetStatusLoopResponse {
                    enabled: loop_enabled,
                    start_measure: loop_start,
                    end_measure: loop_end,
                },
            },
            cache: GetStatusCacheResponse {
                active_render_count: rendering_count,
                pending_count,
                rendering_count,
                ready_count,
                error_count,
                is_updating,
                is_complete: !is_updating,
                cells,
            },
            grid: GetStatusGridResponse {
                tracks: snapshot.grid.tracks,
                measures: snapshot.grid.measures,
            },
        }
    }
}

fn current_measure_index(snapshot: &DawStatusSnapshot) -> Option<usize> {
    match snapshot.play_state {
        DawPlayState::Idle => None,
        DawPlayState::Playing | DawPlayState::Preview => snapshot
            .play_position
            .as_ref()
            .map(|position| position.measure_index),
    }
}

fn current_beat(snapshot: &DawStatusSnapshot) -> Option<u32> {
    if matches!(snapshot.play_state, DawPlayState::Idle)
        || snapshot.beat_count == 0
        || !snapshot.beat_duration_secs.is_finite()
        || snapshot.beat_duration_secs <= 0.0
    {
        return None;
    }
    let position = snapshot.play_position.as_ref()?;
    let raw_beat =
        (position.measure_start.elapsed().as_secs_f64() / snapshot.beat_duration_secs) as u32;
    Some((raw_beat % snapshot.beat_count) + 1)
}

fn play_state_label(play_state: DawPlayState) -> &'static str {
    match play_state {
        DawPlayState::Idle => "idle",
        DawPlayState::Playing => "playing",
        DawPlayState::Preview => "preview",
    }
}

fn cache_state_label(cache_state: &CacheState) -> &'static str {
    match cache_state {
        CacheState::Empty => "empty",
        CacheState::Pending => "pending",
        CacheState::Rendering => "rendering",
        CacheState::Ready => "ready",
        CacheState::Error => "error",
    }
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
    use super::{GetStatusResponse, PostAbRepeatRequest};
    use crate::daw::{AbRepeatState, CacheState, DawPlayState, PlayPosition};

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

    #[test]
    fn get_status_response_serializes_play_and_cache_status() {
        let snapshot = super::super::DawStatusSnapshot {
            play_state: DawPlayState::Playing,
            play_position: Some(PlayPosition {
                measure_index: 2,
                measure_start: std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_millis(100))
                    .unwrap(),
            }),
            beat_count: 4,
            beat_duration_secs: 1.0,
            ab_repeat: AbRepeatState::FixEnd {
                start_measure_index: 0,
                end_measure_index: 2,
            },
            cache: super::super::DawStatusCacheSnapshot {
                cells: vec![
                    vec![CacheState::Empty, CacheState::Ready],
                    vec![CacheState::Pending, CacheState::Rendering],
                ],
                pending_count: 1,
                rendering_count: 1,
                ready_count: 1,
                error_count: 0,
            },
            grid: super::super::DawStatusGridSnapshot {
                tracks: 2,
                measures: 1,
            },
        };

        let body = serde_json::to_value(GetStatusResponse::from(snapshot)).unwrap();

        assert_eq!(body["mode"], "daw");
        assert_eq!(body["play"]["state"], "playing");
        assert_eq!(body["play"]["isPlaying"], true);
        assert_eq!(body["play"]["isPreview"], false);
        assert_eq!(body["play"]["currentMeasure"], 3);
        assert_eq!(body["play"]["currentMeasureIndex"], 2);
        assert_eq!(body["play"]["currentBeat"], 1);
        assert_eq!(body["play"]["loop"]["enabled"], true);
        assert_eq!(body["play"]["loop"]["startMeasure"], 1);
        assert_eq!(body["play"]["loop"]["endMeasure"], 3);
        assert_eq!(body["cache"]["activeRenderCount"], 1);
        assert_eq!(body["cache"]["pendingCount"], 1);
        assert_eq!(body["cache"]["renderingCount"], 1);
        assert_eq!(body["cache"]["readyCount"], 1);
        assert_eq!(body["cache"]["errorCount"], 0);
        assert_eq!(body["cache"]["isUpdating"], true);
        assert_eq!(body["cache"]["isComplete"], false);
        assert_eq!(body["cache"]["cells"][0][0]["state"], "empty");
        assert_eq!(body["cache"]["cells"][0][1]["state"], "ready");
        assert_eq!(body["cache"]["cells"][1][0]["state"], "pending");
        assert_eq!(body["cache"]["cells"][1][1]["state"], "rendering");
        assert_eq!(body["grid"]["tracks"], 2);
        assert_eq!(body["grid"]["measures"], 1);
    }
}
