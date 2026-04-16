use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex, OnceLock,
    },
};

use tiny_http::Method;

use super::{DawApp, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB};
use crate::{config::Config, server::DEFAULT_PORT};
use routes::{
    handle_get_mml, handle_get_patches, handle_options, handle_post_ab_repeat, handle_post_mixer,
    handle_post_mml, handle_post_mode_daw, handle_post_patch, handle_post_play_start,
    handle_post_play_stop, text_response,
};
#[cfg(test)]
use routes::{
    is_allowed_cors_origin, request_origin, with_cors_headers, with_preflight_cors_headers,
};

#[path = "http_server/routes.rs"]
mod routes;

#[derive(Default)]
pub(crate) struct DawHttpState {
    cfg: Option<Arc<Config>>,
    pending_commands: VecDeque<DawHttpCommand>,
    grid_snapshot: Vec<Vec<String>>,
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
    PlayStart,
    PlayStop,
    AbRepeat {
        start_measure: usize,
        end_measure: usize,
    },
}

struct DawHttpServerThreadGuard;

fn active_state_slot() -> &'static Mutex<Option<Arc<Mutex<DawHttpState>>>> {
    static ACTIVE_STATE: OnceLock<Mutex<Option<Arc<Mutex<DawHttpState>>>>> = OnceLock::new();
    ACTIVE_STATE.get_or_init(|| Mutex::new(None))
}

fn server_thread_running() -> &'static AtomicBool {
    static SERVER_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);
    &SERVER_THREAD_RUNNING
}

fn daw_mode_switch_requested() -> &'static AtomicBool {
    static DAW_MODE_SWITCH_REQUESTED: AtomicBool = AtomicBool::new(false);
    &DAW_MODE_SWITCH_REQUESTED
}

fn current_state() -> Option<Arc<Mutex<DawHttpState>>> {
    active_state_slot().lock().unwrap().clone()
}

fn claim_http_server_thread_slot() -> Option<DawHttpServerThreadGuard> {
    server_thread_running()
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .ok()
        .map(|_| DawHttpServerThreadGuard)
}

impl Drop for DawHttpServerThreadGuard {
    fn drop(&mut self) {
        server_thread_running().store(false, Ordering::Release);
    }
}

fn spawn_http_server_thread(server_thread_guard: DawHttpServerThreadGuard) {
    std::thread::spawn(move || {
        let _server_thread_guard = server_thread_guard;
        run_daw_http_server();
    });
}

pub(crate) fn ensure_daw_http_server_thread() {
    let Some(server_thread_guard) = claim_http_server_thread_slot() else {
        return;
    };
    spawn_http_server_thread(server_thread_guard);
}

pub(crate) fn spawn_daw_http_server(state: Arc<Mutex<DawHttpState>>) {
    if state.lock().unwrap().cfg.is_none() {
        return;
    }
    *active_state_slot().lock().unwrap() = Some(state);
    ensure_daw_http_server_thread();
}

pub(crate) fn set_active_http_state_cfg(cfg: Arc<Config>) {
    let state = Arc::new(Mutex::new(DawHttpState {
        cfg: Some(cfg),
        pending_commands: VecDeque::new(),
        grid_snapshot: Vec::new(),
    }));
    spawn_daw_http_server(state);
}

pub(crate) fn deactivate_daw_http_server() {
    *active_state_slot().lock().unwrap() = None;
}

pub(crate) fn request_daw_mode_switch() {
    if current_state().is_none() {
        daw_mode_switch_requested().store(true, Ordering::Release);
    }
}

pub(crate) fn take_daw_mode_switch_request() -> bool {
    daw_mode_switch_requested().swap(false, Ordering::AcqRel)
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
        let path = url.split_once('?').map_or(url.as_str(), |(path, _)| path);

        if method == Method::Options && path == "/mode/daw" {
            handle_options(request);
            continue;
        }
        if method == Method::Post && path == "/mode/daw" {
            handle_post_mode_daw(request);
            continue;
        }

        let Some(state) = current_state() else {
            let _ = request.respond(text_response(
                503,
                "DAW モードがアクティブではありません\n".to_string(),
            ));
            continue;
        };

        match (method, path) {
            (Method::Options, "/mml")
            | (Method::Options, "/mixer")
            | (Method::Options, "/patch")
            | (Method::Options, "/play/start")
            | (Method::Options, "/play/stop")
            | (Method::Options, "/ab-repeat")
            | (Method::Options, "/patches") => handle_options(request),
            (Method::Post, "/mml") => handle_post_mml(request, &state),
            (Method::Post, "/mixer") => handle_post_mixer(request, &state),
            (Method::Post, "/patch") => handle_post_patch(request, &state),
            (Method::Post, "/play/start") => handle_post_play_start(request, &state),
            (Method::Post, "/play/stop") => handle_post_play_stop(request, &state),
            (Method::Post, "/ab-repeat") => handle_post_ab_repeat(request, &state),
            (Method::Get, "/mml") => handle_get_mml(request, &state),
            (Method::Get, "/patches") => handle_get_patches(request, &state),
            _ => {
                let _ = request.respond(text_response(404, "Not Found\n".to_string()));
            }
        }
    }
}

impl DawApp {
    pub(super) fn sync_http_grid_snapshot(&self) {
        let Some(state) = current_state() else {
            return;
        };
        let grid_snapshot = self.data.clone();
        state.lock().unwrap().grid_snapshot = grid_snapshot;
    }

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
                DawHttpCommandKind::PlayStart => self.apply_http_play_start(),
                DawHttpCommandKind::PlayStop => self.apply_http_play_stop(),
                DawHttpCommandKind::AbRepeat {
                    start_measure,
                    end_measure,
                } => self.apply_http_ab_repeat(start_measure, end_measure),
            };
            let _ = command.response_tx.send(result);
        }
    }

    fn ensure_http_grid_size(&mut self, track: usize, measure: usize) -> Result<bool, String> {
        let required_tracks = track
            .checked_add(1)
            .ok_or_else(|| "track index が大きすぎます".to_string())?;
        let required_measures = self.measures.max(measure);
        let current_columns = self
            .measures
            .checked_add(1)
            .ok_or_else(|| "現在の measure 数が大きすぎます".to_string())?;
        let required_columns = required_measures
            .checked_add(1)
            .ok_or_else(|| "measure index が大きすぎます".to_string())?;
        if required_tracks <= self.tracks && required_measures <= self.measures {
            return Ok(false);
        }
        let mut resized = false;

        if required_tracks > self.tracks {
            resized = true;
            self.data.resize_with(required_tracks, || {
                let mut row = Vec::new();
                row.resize_with(current_columns, String::new);
                row
            });
            {
                let mut cache = self.cache.lock().unwrap();
                cache.resize_with(required_tracks, || {
                    vec![super::CellCache::empty(); current_columns]
                });
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
            resized = true;
            for row in &mut self.data {
                row.resize_with(required_columns, String::new);
            }
            {
                let mut cache = self.cache.lock().unwrap();
                for row in cache.iter_mut() {
                    row.resize_with(required_columns, super::CellCache::empty);
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
            self.measures = required_measures;
        }

        for measure_track_mmls in self.play_measure_track_mmls.lock().unwrap().iter_mut() {
            measure_track_mmls.resize_with(self.tracks, String::new);
        }

        Ok(resized)
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
        let grid_resized = self.ensure_http_grid_size(track, self.measures.max(1))?;
        if grid_resized {
            self.sync_http_grid_snapshot();
        }

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

    fn apply_http_play_start(&mut self) -> Result<(), String> {
        let play_state = *self.play_state.lock().unwrap();
        if play_state == super::DawPlayState::Playing {
            self.append_log_line("http: play start (already playing)");
            return Ok(());
        }
        if play_state == super::DawPlayState::Preview {
            self.stop_play();
        }
        self.start_play();
        self.append_log_line("http: play start");
        Ok(())
    }

    fn apply_http_play_stop(&mut self) -> Result<(), String> {
        self.stop_play();
        self.append_log_line("http: play stop");
        Ok(())
    }

    fn apply_http_ab_repeat(
        &mut self,
        start_measure: usize,
        end_measure: usize,
    ) -> Result<(), String> {
        if start_measure == 0 || end_measure == 0 {
            return Err("measA と measB は 1 以上を指定してください".to_string());
        }
        if start_measure > self.measures || end_measure > self.measures {
            return Err(format!(
                "measA と measB は 1..={} の範囲で指定してください",
                self.measures
            ));
        }

        *self.ab_repeat.lock().unwrap() = super::AbRepeatState::FixEnd {
            start_measure_index: start_measure - 1,
            end_measure_index: end_measure - 1,
        };
        self.append_log_line(format!(
            "http: ab-repeat measA={start_measure} measB={end_measure}"
        ));
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/daw/http_server.rs"]
mod tests;
