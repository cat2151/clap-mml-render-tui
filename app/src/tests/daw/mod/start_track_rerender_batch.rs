use tui_textarea::TextArea;

use super::*;
use crate::config::Config;
use crate::daw::{
    AbRepeatState, CacheState, CellCache, DawHistoryPane, DawMode, DawPatchSelectPane, DawPlayState,
};

#[test]
fn start_track_rerender_batch_logs_only_targeted_measures() {
    let tracks = 3;
    let measures = 4;
    let cache_render_workers = 4;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let mut app = DawApp {
        data: vec![vec![String::new(); measures + 1]; tracks],
        cursor_track: 0,
        cursor_measure: 0,
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        textarea: TextArea::default(),
        cfg: Arc::new(Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patches_dirs: None,
            offline_render_workers: cache_render_workers,
            offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
            offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
            offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
            offline_render_server_command: String::new(),
            realtime_audio_backend: crate::config::RealtimeAudioBackend::InProcess,
            realtime_play_server_port: crate::config::DEFAULT_REALTIME_PLAY_SERVER_PORT,
            realtime_play_server_command: String::new(),
        }),
        entry_ptr: 0,
        tracks,
        measures,
        cache: Arc::new(Mutex::new(vec![
            vec![
                CellCache {
                    state: CacheState::Empty,
                    samples: None,
                    rendered_measure_samples: None,
                    generation: 0,
                    rendered_mml_hash: None,
                };
                measures + 1
            ];
            tracks
        ])),
        cache_tx,
        cache_render_workers,
        render_queue: crate::daw::render_queue::RenderQueue::disabled_for_tests(),
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        preview_session: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        preview_sink: Arc::new(Mutex::new(None)),
        realtime_play_server: None,
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
    };
    app.data[1][1] = "c".to_string();
    app.data[1][3] = "e".to_string();
    app.data[1][4] = "g".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Pending;
        cache[1][3].state = CacheState::Pending;
        cache[1][4].state = CacheState::Pending;
    }

    app.start_track_rerender_batch(1, &[1, 3, 4], "random patch update");

    let logs = app.log_lines.lock().unwrap().clone();
    assert!(
        logs.iter()
            .any(|line| line
                == "cache: rerender start track1 meas 1, meas 3〜4 (random patch update)")
    );
    assert!(logs
        .iter()
        .any(|line| line == "cache: rerender reserve track1 meas1 (meas1 -> meas3 -> meas4)"));
    assert!(logs
        .iter()
        .any(|line| line == "cache: rerender reserve track1 meas3 (meas3 -> meas4)"));
    assert!(logs
        .iter()
        .any(|line| line == "cache: rerender reserve track1 meas4 (meas4)"));
    assert_eq!(cache_rx.try_recv().unwrap().measure, 1);
    assert_eq!(cache_rx.try_recv().unwrap().measure, 3);
    assert_eq!(cache_rx.try_recv().unwrap().measure, 4);
    assert!(cache_rx.try_recv().is_err());
}
