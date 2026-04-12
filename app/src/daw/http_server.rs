use std::{
    collections::VecDeque,
    io::Read,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex, Once, OnceLock,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, StatusCode};

use super::{DawApp, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB};
use crate::{config::Config, server::DEFAULT_PORT};

const MAX_JSON_BODY_BYTES: u64 = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(crate) struct DawHttpState {
    cfg: Option<Arc<Config>>,
    pending_commands: VecDeque<DawHttpCommand>,
}

struct DawHttpCommand {
    kind: DawHttpCommandKind,
    response_tx: Sender<Result<(), String>>,
}

enum DawHttpCommandKind {
    Mml {
        track: usize,
        measure: usize,
        mml: String,
    },
    Mixer {
        track: usize,
        db: f64,
    },
    Patch {
        track: usize,
        patch: String,
    },
}

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

fn active_state_slot() -> &'static Mutex<Option<Arc<Mutex<DawHttpState>>>> {
    static ACTIVE_STATE: OnceLock<Mutex<Option<Arc<Mutex<DawHttpState>>>>> = OnceLock::new();
    ACTIVE_STATE.get_or_init(|| Mutex::new(None))
}

fn current_state() -> Option<Arc<Mutex<DawHttpState>>> {
    active_state_slot().lock().unwrap().clone()
}

pub(crate) fn spawn_daw_http_server(state: Arc<Mutex<DawHttpState>>) {
    if state.lock().unwrap().cfg.is_none() {
        return;
    }
    *active_state_slot().lock().unwrap() = Some(state);

    static START_SERVER: Once = Once::new();
    START_SERVER.call_once(|| {
        std::thread::spawn(run_daw_http_server);
    });
}

pub(crate) fn set_active_http_state_cfg(cfg: Arc<Config>) {
    let state = current_state().unwrap_or_else(|| Arc::new(Mutex::new(DawHttpState::default())));
    {
        let mut shared_state = state.lock().unwrap();
        shared_state.cfg = Some(cfg);
    }
    spawn_daw_http_server(state);
}

pub(crate) fn deactivate_daw_http_server() {
    *active_state_slot().lock().unwrap() = None;
}

fn take_pending_http_commands() -> Vec<DawHttpCommand> {
    let Some(state) = current_state() else {
        return Vec::new();
    };
    let mut state = state.lock().unwrap();
    state.pending_commands.drain(..).collect()
}

fn run_daw_http_server() {
    let addr = format!("127.0.0.1:{DEFAULT_PORT}");
    let Ok(server) = tiny_http::Server::http(&addr) else {
        eprintln!("DAW HTTP サーバーの起動に失敗しました: {addr}");
        return;
    };

    for request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let Some(state) = current_state() else {
            let _ = request.respond(text_response(
                503,
                "DAW モードがアクティブではありません\n".to_string(),
            ));
            continue;
        };

        match (method, url.as_str()) {
            (Method::Post, "/mml") => handle_post_mml(request, &state),
            (Method::Post, "/mixer") => handle_post_mixer(request, &state),
            (Method::Post, "/patch") => handle_post_patch(request, &state),
            (Method::Get, "/patches") => handle_get_patches(request, &state),
            _ => {
                let _ = request.respond(text_response(404, "Not Found\n".to_string()));
            }
        }
    }
}

fn handle_post_mml(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    match read_json_body::<PostMmlRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::Mml {
                track: body.track,
                measure: body.measure,
                mml: body.mml,
            },
        ),
        Err((status, message)) => {
            let _ = request.respond(text_response(status, message));
        }
    }
}

fn handle_post_mixer(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    match read_json_body::<PostMixerRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::Mixer {
                track: body.track,
                db: body.db,
            },
        ),
        Err((status, message)) => {
            let _ = request.respond(text_response(status, message));
        }
    }
}

fn handle_post_patch(mut request: Request, state: &Arc<Mutex<DawHttpState>>) {
    match read_json_body::<PostPatchRequest>(&mut request) {
        Ok(body) => respond_command(
            request,
            state,
            DawHttpCommandKind::Patch {
                track: body.track,
                patch: body.patch,
            },
        ),
        Err((status, message)) => {
            let _ = request.respond(text_response(status, message));
        }
    }
}

fn handle_get_patches(request: Request, state: &Arc<Mutex<DawHttpState>>) {
    let cfg = {
        let state = state.lock().unwrap();
        state.cfg.clone()
    };
    let Some(cfg) = cfg else {
        let _ = request.respond(text_response(
            503,
            "DAW モードがアクティブではありません\n".to_string(),
        ));
        return;
    };

    if !crate::patches::has_configured_patch_dirs(cfg.as_ref()) {
        let empty = Vec::<String>::new();
        let _ = request.respond(json_response(200, &empty));
        return;
    }

    match crate::patches::collect_patch_pairs(cfg.as_ref()) {
        Ok(pairs) => {
            let patches = pairs
                .into_iter()
                .map(|(patch_name, _)| patch_name)
                .collect::<Vec<_>>();
            let _ = request.respond(json_response(200, &patches));
        }
        Err(error) => {
            let _ = request.respond(text_response(
                500,
                format!("patch 一覧の取得に失敗しました: {error}\n"),
            ));
        }
    }
}

fn respond_command(request: Request, state: &Arc<Mutex<DawHttpState>>, kind: DawHttpCommandKind) {
    let (response_tx, response_rx) = mpsc::channel();
    {
        let mut state = state.lock().unwrap();
        state
            .pending_commands
            .push_back(DawHttpCommand { kind, response_tx });
    }

    match response_rx.recv_timeout(COMMAND_TIMEOUT) {
        Ok(Ok(())) => {
            let _ = request.respond(json_response(200, &JsonStatusResponse { status: "ok" }));
        }
        Ok(Err(message)) => {
            let _ = request.respond(text_response(400, format!("{message}\n")));
        }
        Err(_) => {
            let _ = request.respond(text_response(
                504,
                "DAW 反映待ちがタイムアウトしました\n".to_string(),
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

fn text_response(status: u16, body: String) -> Response<std::io::Cursor<Vec<u8>>> {
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

impl DawApp {
    pub(super) fn apply_pending_http_commands(&mut self) {
        for command in take_pending_http_commands() {
            let result = match command.kind {
                DawHttpCommandKind::Mml {
                    track,
                    measure,
                    mml,
                } => self.apply_http_mml(track, measure, &mml),
                DawHttpCommandKind::Mixer { track, db } => self.apply_http_mixer(track, db),
                DawHttpCommandKind::Patch { track, patch } => self.apply_http_patch(track, &patch),
            };
            let _ = command.response_tx.send(result);
        }
    }

    fn ensure_http_grid_size(&mut self, track: usize, measure: usize) -> Result<(), String> {
        let required_tracks = track
            .checked_add(1)
            .ok_or_else(|| "track index が大きすぎます".to_string())?;
        let required_measures = self.measures.max(measure);
        if required_tracks <= self.tracks && required_measures <= self.measures {
            return Ok(());
        }

        if required_tracks > self.tracks {
            let columns = self.measures + 1;
            self.data.resize_with(required_tracks, || {
                let mut row = Vec::new();
                row.resize_with(columns, String::new);
                row
            });
            {
                let mut cache = self.cache.lock().unwrap();
                cache.resize_with(required_tracks, || vec![super::CellCache::empty(); columns]);
            }
            self.solo_tracks.resize(required_tracks, false);
            self.track_volumes_db.resize(required_tracks, 0);
            self.play_track_gains
                .lock()
                .unwrap()
                .resize(required_tracks, 0.0);
            self.track_rerender_batches
                .lock()
                .unwrap()
                .resize(required_tracks, None);
            self.tracks = required_tracks;
        }

        if required_measures > self.measures {
            let additional = required_measures - self.measures;
            for row in &mut self.data {
                row.resize_with(required_measures + 1, String::new);
            }
            {
                let mut cache = self.cache.lock().unwrap();
                for row in cache.iter_mut() {
                    row.resize_with(required_measures + 1, super::CellCache::empty);
                }
            }
            self.play_measure_mmls
                .lock()
                .unwrap()
                .resize_with(required_measures, String::new);
            self.play_measure_track_mmls
                .lock()
                .unwrap()
                .resize_with(required_measures, || vec![String::new(); self.tracks]);
            if additional > 0 {
                self.measures = required_measures;
            }
        }

        for measure_track_mmls in self.play_measure_track_mmls.lock().unwrap().iter_mut() {
            measure_track_mmls.resize_with(self.tracks, String::new);
        }

        Ok(())
    }

    fn apply_http_mml(&mut self, track: usize, measure: usize, mml: &str) -> Result<(), String> {
        if measure == 0 {
            return Err("measure は 1 以上を指定してください".to_string());
        }
        self.ensure_http_grid_size(track, measure)?;
        self.commit_insert_cell(track, measure, mml);
        self.save();
        self.sync_playback_mml_state();
        self.append_log_line(format!("http: mml track={track} meas={measure}"));
        Ok(())
    }

    fn apply_http_mixer(&mut self, track: usize, db: f64) -> Result<(), String> {
        if !db.is_finite() {
            return Err("db は有限な数値を指定してください".to_string());
        }
        if track < FIRST_PLAYABLE_TRACK {
            return Err("mixer は演奏トラックでのみ使用できます".to_string());
        }
        self.ensure_http_grid_size(track, self.measures.max(1))?;

        let rounded_db = db.round() as i32;
        let clamped_db = rounded_db.clamp(MIXER_MIN_DB, MIXER_MAX_DB);
        let current_db = self.track_volume_db(track);
        if clamped_db != current_db {
            let _ = self.adjust_track_volume_db(track, clamped_db - current_db);
            self.save();
            self.sync_playback_mml_state();
        }
        self.append_log_line(format!("http: mixer track={track} db={clamped_db}"));
        Ok(())
    }

    fn apply_http_patch(&mut self, track: usize, patch_name: &str) -> Result<(), String> {
        if track < FIRST_PLAYABLE_TRACK {
            return Err("patch は演奏トラックでのみ使用できます".to_string());
        }
        self.ensure_http_grid_size(track, self.measures.max(1))?;

        let patch_pairs = crate::patches::collect_patch_pairs(self.cfg.as_ref())
            .map_err(|error| format!("patch 一覧の取得に失敗しました: {error}"))?;
        let display_patch_name =
            crate::patches::resolve_display_patch_name(&patch_pairs, patch_name)
                .ok_or_else(|| format!("patch が見つかりません: {patch_name}"))?;
        let patch_json = Self::build_patch_json(&display_patch_name);
        self.commit_insert_cell(track, 0, &patch_json);
        self.save();
        self.sync_playback_mml_state();
        self.append_log_line(format!(
            "http: patch track={track} patch={display_patch_name}"
        ));
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/daw/http_server.rs"]
mod tests;
