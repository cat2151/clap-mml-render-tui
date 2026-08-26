use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex, OnceLock},
};

use ratatui_textarea::TextArea;

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
use crate::{AbRepeatState, CacheState, CellCache, DawApp, DawMode, DawPlayState};
use cmrt_runtime::Config;

fn build_test_app(cfg: Config) -> DawApp {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        workspace_kind: crate::WorkspaceKind::Persistent,
        daily_page_date: None,
        config_app_dir: None,
        editor: crate::editor::DawEditorState::new(
            vec![vec![String::new(); measures + 1]; tracks],
            1,
            1,
            tracks,
            measures,
        ),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: cmrt_tui_core::sound_check_guide::SoundCheckGuide::new(None),
        textarea: TextArea::default(),
        cfg: Arc::new(cfg),
        plugin_entries: cmrt_offline_render::PluginEntries::none(),
        cache: Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); measures + 1];
            tracks
        ])),
        cache_tx,
        cache_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        render_queue: crate::render_queue::RenderQueue::disabled_for_tests(),
        playback: crate::playback_runtime::DawPlaybackRuntime::for_test(tracks, measures),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        overlays: crate::overlays::DawOverlays::new(1),
        patch_phrase_store: cmrt_history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,

        random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
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
        loop_dirs: Vec::new(),
        loop_categories: cmrt_runtime::default_loop_categories(),
        offline_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
        offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::InProcess,
        realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        ..Default::default()
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

mod apply_pending_commands;
mod cors;
mod lifecycle;
mod request_headers;
mod snapshot_queries;
