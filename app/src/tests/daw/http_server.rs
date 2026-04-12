use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex},
};

use tui_textarea::TextArea;

use super::{
    active_state_slot, deactivate_daw_http_server, DawHttpCommand, DawHttpCommandKind, DawHttpState,
};
use crate::{
    config::Config,
    daw::{
        AbRepeatState, CellCache, DawApp, DawHistoryPane, DawMode, DawPatchSelectPane, DawPlayState,
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

#[test]
fn apply_pending_http_commands_updates_mml_and_expands_grid() {
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_mml");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = default_config();
    let state = Arc::new(Mutex::new(DawHttpState {
        cfg: Some(Arc::new(cfg.clone())),
        pending_commands: VecDeque::new(),
    }));
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::Mml {
            track: 3,
            measure: 4,
            mml: "l8cde".to_string(),
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(app.tracks, 4);
    assert_eq!(app.measures, 4);
    assert_eq!(app.data[3][4], "l8cde");
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_commands_updates_mixer_gain() {
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_mixer");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = default_config();
    let state = Arc::new(Mutex::new(DawHttpState {
        cfg: Some(Arc::new(cfg.clone())),
        pending_commands: VecDeque::new(),
    }));
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::Mixer { track: 1, db: -6.0 });

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(app.track_volume_db(1), -6);
    assert_eq!(
        app.play_track_gains.lock().unwrap()[1],
        10.0f32.powf(-6.0 / 20.0)
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_commands_updates_patch_init_cell() {
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_patch");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(tmp.join("Pads")).unwrap();
    std::fs::write(tmp.join("Pads").join("Factory Pad.fxp"), b"dummy").unwrap();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let mut cfg = default_config();
    cfg.patches_dirs = Some(vec![tmp.to_string_lossy().into_owned()]);
    let state = Arc::new(Mutex::new(DawHttpState {
        cfg: Some(Arc::new(cfg.clone())),
        pending_commands: VecDeque::new(),
    }));
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::Patch {
            track: 1,
            patch: "Pads/Factory Pad.fxp".to_string(),
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(
        app.data[1][0],
        DawApp::build_patch_json("Pads/Factory Pad.fxp")
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}
