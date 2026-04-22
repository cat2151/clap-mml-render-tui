use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex, OnceLock},
};

use tui_textarea::TextArea;

use super::routes::{
    get_snapshot_mml, get_snapshot_mmls, get_status_snapshot, if_none_match_matches,
    parse_get_mml_query, request_header_value, snapshot_mmls_etag, RequestHeaderName,
};
use super::{
    claim_http_server_thread_slot, deactivate_daw_http_server, is_allowed_cors_origin,
    request_daw_mode_switch, request_origin, set_test_active_http_state_for_current_thread,
    take_daw_mode_switch_request, with_cors_headers, with_preflight_cors_headers, DawHttpCommand,
    DawHttpCommandKind, DawHttpState,
};
use crate::{
    config::Config,
    daw::{
        AbRepeatState, CacheState, CellCache, DawApp, DawHistoryPane, DawMode, DawPatchSelectPane,
        DawPlayState,
    },
};

fn build_test_app(cfg: Config) -> DawApp {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        data: vec![vec![String::new(); measures + 1]; tracks],
        cursor_track: 1,
        cursor_measure: 1,
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        textarea: TextArea::default(),
        cfg: Arc::new(cfg),
        entry_ptr: 0,
        tracks,
        measures,
        cache: Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); measures + 1];
            tracks
        ])),
        cache_tx,
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        render_queue: crate::daw::render_queue::RenderQueue::disabled_for_tests(),
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        preview_session: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        preview_sink: Arc::new(Mutex::new(None)),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(AbRepeatState::Off)),
        overlay_preview_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
        play_measure_track_mmls: Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures])),
        play_measure_samples: Arc::new(Mutex::new(0)),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        mixer_cursor_track: 1,
        play_track_gains: Arc::new(Mutex::new(vec![0.0; tracks])),
        yank_buffer: None,
        normal_pending_delete: false,
        normal_paste_undo: None,
        patch_phrase_store: crate::history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,
        history_overlay_patch_name: None,
        history_overlay_query: String::new(),
        history_overlay_query_textarea: crate::text_input::new_single_line_textarea(""),
        history_overlay_history_cursor: 0,
        history_overlay_favorites_cursor: 0,
        history_overlay_focus: DawHistoryPane::History,
        history_overlay_filter_active: false,
        patch_all: Vec::new(),
        patch_query: String::new(),
        patch_query_textarea: crate::text_input::new_single_line_textarea(""),
        patch_query_before_input: String::new(),
        patch_filtered: Vec::new(),
        patch_cursor: 0,
        patch_favorite_items: Vec::new(),
        patch_favorites_cursor: 0,
        patch_select_focus: DawPatchSelectPane::Patches,
        patch_select_filter_active: false,
        random_patch_decks: crate::random::RandomIndexDecks::default(),
    }
}

fn default_config() -> Config {
    Config {
        plugin_path: String::new(),
        input_midi: String::new(),
        output_midi: String::new(),
        output_wav: String::new(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: None,
        offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
        offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
    }
}

fn enqueue_command(
    state: &Arc<Mutex<DawHttpState>>,
    kind: DawHttpCommandKind,
) -> mpsc::Receiver<Result<(), String>> {
    let (response_tx, response_rx) = mpsc::channel();
    state
        .lock()
        .unwrap()
        .pending_commands
        .push_back(DawHttpCommand { kind, response_tx });
    response_rx
}

fn build_http_state(cfg: Config) -> Arc<Mutex<DawHttpState>> {
    Arc::new(Mutex::new(DawHttpState {
        cfg: Some(Arc::new(cfg)),
        pending_commands: VecDeque::new(),
        grid_snapshot: Vec::new(),
        status_snapshot: None,
    }))
}

fn activate_http_state(state: Arc<Mutex<DawHttpState>>) {
    let _ = set_test_active_http_state_for_current_thread(Some(state));
}

/// Serializes tests that touch DAW HTTP server globals such as
/// `active_state_slot`, the server thread slot, and the mode-switch flag.
/// Without this, parallel test execution can race and make unrelated
/// assertions flaky.
fn http_server_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_http_server_test_state() -> std::sync::MutexGuard<'static, ()> {
    http_server_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[path = "http_server/apply_pending_commands.rs"]
mod apply_pending_commands;
#[path = "http_server/request_and_cors.rs"]
mod request_and_cors;
#[path = "http_server/snapshot_queries.rs"]
mod snapshot_queries;
