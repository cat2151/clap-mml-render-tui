use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex, OnceLock,
    },
};

#[cfg(test)]
use std::cell::{Cell, RefCell};

use tiny_http::Method;

use super::{
    AbRepeatState, CacheState, DawApp, DawPlayState, PlayPosition, FIRST_PLAYABLE_TRACK,
    MIXER_MAX_DB, MIXER_MIN_DB,
};
use crate::{config::Config, server::DEFAULT_PORT};
use routes::{
    handle_get_mml, handle_get_mmls, handle_get_patches, handle_get_status, handle_options,
    handle_post_ab_repeat, handle_post_mixer, handle_post_mml, handle_post_mode_daw,
    handle_post_patch, handle_post_patch_random, handle_post_play_start, handle_post_play_stop,
    text_response,
};
#[cfg(test)]
use routes::{
    is_allowed_cors_origin, request_origin, with_cors_headers, with_preflight_cors_headers,
};

#[path = "http_server/app.rs"]
mod app;
#[path = "http_server/routes.rs"]
mod routes;

#[derive(Default)]
pub(crate) struct DawHttpState {
    cfg: Option<Arc<Config>>,
    pending_commands: VecDeque<DawHttpCommand>,
    grid_snapshot: Vec<Vec<String>>,
    status_snapshot: Option<DawStatusSnapshot>,
}

#[derive(Clone)]
struct DawStatusSnapshot {
    play_state: DawPlayState,
    play_position: Option<PlayPosition>,
    beat_count: u32,
    beat_duration_secs: f64,
    ab_repeat: AbRepeatState,
    cache: DawStatusCacheSnapshot,
    grid: DawStatusGridSnapshot,
}

#[derive(Clone)]
struct DawStatusCacheSnapshot {
    cells: Vec<Vec<CacheState>>,
    pending_count: usize,
    rendering_count: usize,
    ready_count: usize,
    error_count: usize,
}

#[derive(Clone, Copy)]
struct DawStatusGridSnapshot {
    tracks: usize,
    measures: usize,
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
    RandomPatch {
        track: usize,
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

#[cfg(test)]
thread_local! {
    static TEST_ACTIVE_STATE: RefCell<Option<Arc<Mutex<DawHttpState>>>> = const { RefCell::new(None) };
    static TEST_DAW_MODE_SWITCH_REQUESTED: Cell<bool> = const { Cell::new(false) };
}

fn server_thread_running() -> &'static AtomicBool {
    static SERVER_THREAD_RUNNING: AtomicBool = AtomicBool::new(false);
    &SERVER_THREAD_RUNNING
}

#[cfg(not(test))]
fn daw_mode_switch_requested() -> &'static AtomicBool {
    static DAW_MODE_SWITCH_REQUESTED: AtomicBool = AtomicBool::new(false);
    &DAW_MODE_SWITCH_REQUESTED
}

fn current_state() -> Option<Arc<Mutex<DawHttpState>>> {
    #[cfg(test)]
    if let Some(state) = test_active_http_state_for_current_thread() {
        return Some(state);
    }
    active_state_slot().lock().unwrap().clone()
}

#[cfg(test)]
pub(crate) fn set_test_active_http_state_for_current_thread(
    state: Option<Arc<Mutex<DawHttpState>>>,
) -> Option<Arc<Mutex<DawHttpState>>> {
    TEST_ACTIVE_STATE.with(|slot| slot.replace(state))
}

#[cfg(test)]
fn test_active_http_state_for_current_thread() -> Option<Arc<Mutex<DawHttpState>>> {
    TEST_ACTIVE_STATE.with(|slot| slot.borrow().clone())
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
    #[cfg(test)]
    let _ = set_test_active_http_state_for_current_thread(Some(Arc::clone(&state)));
    *active_state_slot().lock().unwrap() = Some(state);
    ensure_daw_http_server_thread();
}

pub(crate) fn set_active_http_state_cfg(cfg: Arc<Config>) {
    let state = Arc::new(Mutex::new(DawHttpState {
        cfg: Some(cfg),
        pending_commands: VecDeque::new(),
        grid_snapshot: Vec::new(),
        status_snapshot: None,
    }));
    spawn_daw_http_server(state);
}

pub(crate) fn deactivate_daw_http_server() {
    #[cfg(test)]
    let _ = set_test_active_http_state_for_current_thread(None);
    *active_state_slot().lock().unwrap() = None;
}

pub(crate) fn request_daw_mode_switch() {
    if current_state().is_none() {
        #[cfg(test)]
        TEST_DAW_MODE_SWITCH_REQUESTED.with(|requested| requested.set(true));
        #[cfg(not(test))]
        daw_mode_switch_requested().store(true, Ordering::Release);
    }
}

pub(crate) fn take_daw_mode_switch_request() -> bool {
    #[cfg(test)]
    {
        TEST_DAW_MODE_SWITCH_REQUESTED.with(|requested| requested.replace(false))
    }
    #[cfg(not(test))]
    {
        daw_mode_switch_requested().swap(false, Ordering::AcqRel)
    }
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
            | (Method::Options, "/patch/random")
            | (Method::Options, "/play/start")
            | (Method::Options, "/play/stop")
            | (Method::Options, "/ab-repeat")
            | (Method::Options, "/mmls")
            | (Method::Options, "/patches")
            | (Method::Options, "/status") => handle_options(request),
            (Method::Post, "/mml") => handle_post_mml(request, &state),
            (Method::Post, "/mixer") => handle_post_mixer(request, &state),
            (Method::Post, "/patch") => handle_post_patch(request, &state),
            (Method::Post, "/patch/random") => handle_post_patch_random(request, &state),
            (Method::Post, "/play/start") => handle_post_play_start(request, &state),
            (Method::Post, "/play/stop") => handle_post_play_stop(request, &state),
            (Method::Post, "/ab-repeat") => handle_post_ab_repeat(request, &state),
            (Method::Get, "/mml") => handle_get_mml(request, &state),
            (Method::Get, "/mmls") => handle_get_mmls(request, &state),
            (Method::Get, "/patches") => handle_get_patches(request, &state),
            (Method::Get, "/status") => handle_get_status(request, &state),
            _ => {
                let _ = request.respond(text_response(404, "Not Found\n".to_string()));
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/daw/http_server.rs"]
mod tests;
