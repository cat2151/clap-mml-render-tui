use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex},
};

use tui_textarea::TextArea;

use super::routes::{get_snapshot_mml, parse_get_mml_query};
use super::{
    active_state_slot, claim_http_server_thread_slot, deactivate_daw_http_server,
    is_allowed_cors_origin, request_daw_mode_switch, request_origin, take_daw_mode_switch_request,
    with_cors_headers, with_preflight_cors_headers, DawHttpCommand, DawHttpCommandKind,
    DawHttpState,
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
    }))
}

fn activate_http_state(state: Arc<Mutex<DawHttpState>>) {
    *active_state_slot().lock().unwrap() = Some(state);
}

#[test]
fn apply_pending_http_commands_updates_mml_and_expands_grid() {
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_updates_mml");
    std::fs::remove_dir_all(&tmp).ok();
    let _guard = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = default_config();
    let state = build_http_state(cfg.clone());
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
    assert_eq!(state.lock().unwrap().grid_snapshot[3][4], "l8cde");
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
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::Mixer { track: 4, db: -6.0 });

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(app.track_volume_db(4), -6);
    assert_eq!(
        app.play_track_gains.lock().unwrap()[4],
        10.0f32.powf(-6.0 / 20.0)
    );
    assert_eq!(state.lock().unwrap().grid_snapshot.len(), 5);
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
    let state = build_http_state(cfg.clone());
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
    assert_eq!(
        state.lock().unwrap().grid_snapshot[1][0],
        DawApp::build_patch_json("Pads/Factory Pad.fxp")
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_commands_starts_play() {
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStart);

    let mut app = build_test_app(cfg);
    app.data[1][1] = "l8c".to_string();
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "play: start"));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "http: play start"));

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_start_while_playing_is_noop() {
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStart);

    let mut app = build_test_app(cfg);
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Playing
    ));
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("http: play start (already playing)")
    );

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_start_without_playable_data_returns_error() {
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStart);

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(
        response_rx.try_recv().unwrap(),
        Err("再生可能なデータがありません".to_string())
    );
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("http: play start (no playable data)")
    );

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_stops_play() {
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(&state, DawHttpCommandKind::PlayStop);

    let mut app = build_test_app(cfg);
    *app.play_state.lock().unwrap() = DawPlayState::Playing;
    app.apply_pending_http_commands();

    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Idle
    ));
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "play: stop"));
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "http: play stop"));

    deactivate_daw_http_server();
}

#[test]
fn apply_pending_http_commands_updates_ab_repeat_range() {
    let cfg = default_config();
    let state = build_http_state(cfg.clone());
    *active_state_slot().lock().unwrap() = Some(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::AbRepeat {
            start_measure: 1,
            end_measure: 2,
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));

    deactivate_daw_http_server();
}

#[test]
fn request_origin_extracts_origin_header() {
    let header = tiny_http::Header::from_bytes("Origin", "https://cat2151.github.io").unwrap();

    assert_eq!(
        request_origin(&[header]),
        Some("https://cat2151.github.io".to_string())
    );
    assert_eq!(request_origin(&[]), None);
}

#[test]
fn is_allowed_cors_origin_accepts_known_origins() {
    assert!(is_allowed_cors_origin("https://cat2151.github.io"));
    assert!(is_allowed_cors_origin("http://localhost:5173"));
    assert!(!is_allowed_cors_origin("https://example.com"));
}

#[test]
fn with_cors_headers_adds_origin_and_vary_headers() {
    let response = with_cors_headers(
        tiny_http::Response::from_string("ok"),
        Some("https://cat2151.github.io"),
    );

    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Allow-Origin")
            && header.value.as_str() == "https://cat2151.github.io"));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Vary") && header.value.as_str() == "Origin"));
}

#[test]
fn with_preflight_cors_headers_adds_preflight_headers() {
    let response = with_preflight_cors_headers(
        tiny_http::Response::from_string(""),
        Some("http://localhost:5173"),
    );

    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Allow-Methods")));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Allow-Headers")));
    assert!(response
        .headers()
        .iter()
        .any(|header| header.field.equiv("Access-Control-Max-Age")));
}

#[test]
fn claim_http_server_thread_slot_is_reusable_after_drop() {
    let first_guard = claim_http_server_thread_slot().expect("first claim should succeed");
    assert!(
        claim_http_server_thread_slot().is_none(),
        "second concurrent claim should fail"
    );
    drop(first_guard);
    assert!(
        claim_http_server_thread_slot().is_some(),
        "slot should be reusable after guard drop"
    );
}

#[test]
fn daw_mode_switch_request_is_consumed_once() {
    deactivate_daw_http_server();
    assert!(!take_daw_mode_switch_request());

    request_daw_mode_switch();

    assert!(take_daw_mode_switch_request());
    assert!(!take_daw_mode_switch_request());
}

#[test]
fn daw_mode_switch_request_is_ignored_while_daw_is_active() {
    deactivate_daw_http_server();
    assert!(!take_daw_mode_switch_request());
    activate_http_state(build_http_state(default_config()));

    request_daw_mode_switch();

    assert!(!take_daw_mode_switch_request());
    deactivate_daw_http_server();
    assert!(!take_daw_mode_switch_request());
}

#[test]
fn apply_http_mml_rejects_measure_index_overflow() {
    let cfg = default_config();
    let mut app = build_test_app(cfg);

    let result = app.apply_http_mml(1, usize::MAX, "c");

    assert_eq!(result, Err("measure index が大きすぎます".to_string()));
}

#[test]
fn apply_http_ab_repeat_rejects_init_column_and_out_of_range_measure() {
    let cfg = default_config();
    let mut app = build_test_app(cfg);

    assert_eq!(
        app.apply_http_ab_repeat(0, 1),
        Err("measA と measB は 1 以上を指定してください".to_string())
    );
    assert_eq!(
        app.apply_http_ab_repeat(1, 3),
        Err("measA と measB は 1..=2 の範囲で指定してください".to_string())
    );
}

#[test]
fn parse_get_mml_query_accepts_measure_alias_and_zero() {
    assert_eq!(parse_get_mml_query("/mml?track=2&measure=0"), Ok((2, 0)));
    assert_eq!(parse_get_mml_query("/mml?track=2&meas=0"), Ok((2, 0)));
}

#[test]
fn parse_get_mml_query_rejects_missing_or_invalid_values() {
    assert_eq!(
        parse_get_mml_query("/mml?track=2"),
        Err((400, "track と measure を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=&measure=0"),
        Err((400, "track を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=2&measure="),
        Err((400, "measure を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=abc&measure=0"),
        Err((400, "track は 0 以上の整数を指定してください\n".to_string()))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=2&measure=abc"),
        Err((
            400,
            "measure は 0 以上の整数を指定してください\n".to_string()
        ))
    );
    assert_eq!(
        parse_get_mml_query("/mml?track=2&meas=abc"),
        Err((
            400,
            "measure は 0 以上の整数を指定してください\n".to_string()
        ))
    );
}

#[test]
fn get_snapshot_mml_rejects_unready_and_out_of_range_requests() {
    let state = DawHttpState::default();
    assert_eq!(
        get_snapshot_mml(&state, 0, 0),
        Err((503, "DAW データの準備中です\n".to_string()))
    );

    let state = DawHttpState {
        cfg: None,
        pending_commands: VecDeque::new(),
        grid_snapshot: vec![vec!["t120".to_string()]],
    };
    assert_eq!(
        get_snapshot_mml(&state, 1, 0),
        Err((
            404,
            "指定された track/measure は範囲外です: track=1, measure=0\n".to_string()
        ))
    );
    assert_eq!(get_snapshot_mml(&state, 0, 0), Ok("t120".to_string()));
}
