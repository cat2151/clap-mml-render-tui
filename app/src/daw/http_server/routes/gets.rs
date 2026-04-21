use super::*;

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
    measure_elapsed_ms: Option<u64>,
    measure_duration_ms: Option<u64>,
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

pub(in crate::daw::http_server) fn handle_get_mml(
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

pub(in crate::daw::http_server) fn handle_get_mmls(
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

pub(in crate::daw::http_server) fn handle_get_patches(
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

pub(in crate::daw::http_server) fn handle_get_status(
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

impl From<DawStatusSnapshot> for GetStatusResponse {
    fn from(snapshot: DawStatusSnapshot) -> Self {
        let current_measure_index = current_measure_index(&snapshot);
        let current_measure = current_measure_index.map(|measure_index| measure_index + 1);
        let current_beat = current_beat(&snapshot);
        let measure_elapsed_ms = current_measure_elapsed_ms(&snapshot);
        let measure_duration_ms = current_measure_duration_ms(&snapshot);
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
                measure_elapsed_ms,
                measure_duration_ms,
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

fn current_measure_elapsed_ms(snapshot: &DawStatusSnapshot) -> Option<u64> {
    if matches!(snapshot.play_state, DawPlayState::Idle) {
        return None;
    }
    let position = snapshot.play_position.as_ref()?;
    Some(duration_millis_u64(position.measure_start.elapsed()))
}

fn current_measure_duration_ms(snapshot: &DawStatusSnapshot) -> Option<u64> {
    if matches!(snapshot.play_state, DawPlayState::Idle) {
        return None;
    }
    let position = snapshot.play_position.as_ref()?;
    Some(duration_millis_u64(position.measure_duration))
}

fn duration_millis_u64(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
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

#[cfg(test)]
mod tests {
    use super::GetStatusResponse;
    use crate::daw::{AbRepeatState, CacheState, DawPlayState, PlayPosition};

    #[test]
    fn get_status_response_serializes_play_and_cache_status() {
        let snapshot = super::super::super::DawStatusSnapshot {
            play_state: DawPlayState::Playing,
            play_position: Some(PlayPosition {
                measure_index: 2,
                measure_start: std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_millis(100))
                    .unwrap(),
                measure_duration: std::time::Duration::from_millis(4_000),
            }),
            beat_count: 4,
            beat_duration_secs: 1.0,
            ab_repeat: AbRepeatState::FixEnd {
                start_measure_index: 0,
                end_measure_index: 2,
            },
            cache: super::super::super::DawStatusCacheSnapshot {
                cells: vec![
                    vec![CacheState::Empty, CacheState::Ready],
                    vec![CacheState::Pending, CacheState::Rendering],
                ],
                pending_count: 1,
                rendering_count: 1,
                ready_count: 1,
                error_count: 0,
            },
            grid: super::super::super::DawStatusGridSnapshot {
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
        assert!(body["play"]["measureElapsedMs"].as_u64().unwrap() >= 100);
        assert_eq!(body["play"]["measureDurationMs"], 4_000);
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
